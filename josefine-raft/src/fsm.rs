use std::fmt;

use slog::Logger;
use tokio::sync::mpsc;

use josefine_core::error::Result;
use crate::{
    raft::{Entry, EntryType, LogIndex},
    rpc,
};
use crate::rpc::{Message, Address, Response};
use crate::raft::Command;

pub trait Fsm: Send + Sync + fmt::Debug {
    fn transition(&mut self, data: Vec<u8>) -> Result<Vec<u8>>;
    fn query(&mut self, data: Vec<u8>) -> Result<Vec<u8>>;
}

#[derive(Debug)]
pub enum Instruction {
    Drive { entry: Entry },
    Query(Vec<u8>),
}

pub struct Driver<T: Fsm> {
    logger: Logger,
    fsm_rx: mpsc::UnboundedReceiver<Instruction>,
    rpc_tx: mpsc::UnboundedSender<rpc::Message>,
    applied_idx: LogIndex,
    fsm: T,
}
impl<T: Fsm> Driver<T> {
    pub fn new(
        logger: Logger,
        fsm_rx: mpsc::UnboundedReceiver<Instruction>,
        rpc_tx: mpsc::UnboundedSender<rpc::Message>,
        fsm: T,
    ) -> Self {
        Self {
            logger,
            fsm_rx,
            rpc_tx,
            fsm,
            applied_idx: 0,
        }
    }

    pub async fn run(mut self, mut shutdown: tokio::sync::broadcast::Receiver<()>) -> Result<T> {
        debug!(self.logger, "starting driver"; "fsm" => format!("{:?}", self.fsm));
        loop {
            tokio::select! {
                _ = shutdown.recv() => break,

                Some(instruction) = self.fsm_rx.recv() => {
                    self.exec(instruction).await?;
                }
            }
        }

        Ok(self.fsm)
    }

    pub async fn exec(&mut self, instruction: Instruction) -> Result<()> {
        debug!(self.logger, "exec"; "instruction" => format!("{:?}", &instruction));

        match instruction {
            Instruction::Drive { entry } => {
                if let EntryType::Entry { data } = entry.entry_type {
                    self.fsm.transition(data)?;
                }
            },
            Instruction::Query(data) => {
                let res = self.fsm.query(data)?;
                self.rpc_tx.send(Message {
                    to: Address::Local,
                    from: Address::Local,
                    command: Command::ClientResponse {
                        id: vec![],
                        res: Ok(Response::State(res)),
                    }
                })?;
            },
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc::unbounded_channel;

    use crate::error::RaftError;

    use super::*;
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum TestState {
        A,
        B,
    }
    #[derive(Debug, PartialEq, Eq, Clone)]
    struct TestFsm {
        state: TestState,
    }

    impl TestFsm {
        pub fn new() -> Self {
            Self {
                state: TestState::A,
            }
        }
    }

    impl Fsm for TestFsm {
        fn transition(&mut self, input: Vec<u8>) -> Result<Vec<u8>> {
            let state = std::str::from_utf8(&input).unwrap();
            match state {
                "A" => self.state = TestState::A,
                "B" => self.state = TestState::B,
                _ => panic!(),
            };

            Ok(Vec::new())
        }

        fn query(&mut self, _: Vec<u8>) -> Result<Vec<u8>> {
            let state = match self.state {
                TestState::A => "A",
                TestState::B => "B",
            };
            Ok(String::into_bytes(state.to_string()))
        }
    }

    #[tokio::test]
    async fn transition() -> Result<()> {
        let fsm = TestFsm::new();

        let (tx, rx) = unbounded_channel();
        let (rpc_tx, rpc_rx) = unbounded_channel();
        let driver = Driver::new(crate::logger::get_root_logger().new(o!()), rx, rpc_tx, fsm);

        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        tx.send(Instruction::Drive {
            entry: Entry {
                entry_type: crate::raft::EntryType::Entry {
                    data: "B".as_bytes().to_owned(),
                },
                term: 0,
                index: 0,
            },
        }).map_err(|err| RaftError::from(err))?;

        let (join, _) = tokio::join!(
            tokio::spawn(driver.run(shutdown_rx)),
            tokio::spawn(async move { shutdown_tx.send(()).unwrap() }),
        );
        let fsm = join??;

        assert_eq!(fsm.state, TestState::B);

        Ok(())
    }

    #[tokio::test]
    async fn query() -> Result<()> {
        let fsm = TestFsm::new();

        let (tx, rx) = unbounded_channel();
        let (rpc_tx, mut rpc_rx) = unbounded_channel();
        let driver = Driver::new(crate::logger::get_root_logger().new(o!()), rx, rpc_tx, fsm);

        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
        tx.send(Instruction::Query(vec![])).map_err(|err| RaftError::from(err))?;

        let (_, join, _) = tokio::join!(
            tokio::spawn(driver.run(shutdown_rx)),
            tokio::spawn(async move { rpc_rx.recv().await }),
            tokio::spawn(async move { shutdown_tx.send(()).unwrap() }),
        );
        let res = join?.unwrap();

        if let Command::ClientResponse { res, .. } = res.command {
            if let Response::State(data) = res? {
                assert_eq!("A", String::from_utf8(data).unwrap())
            } else { panic!() }
        } else { panic!() };
        Ok(())
    }
}
