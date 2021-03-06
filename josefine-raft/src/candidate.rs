use std::time::Instant;

use slog::Logger;
use josefine_core::error::Result;

use crate::election::{Election, ElectionStatus};
use crate::follower::Follower;
use crate::leader::Leader;
use crate::progress::ReplicationProgress;
use crate::raft::{Command, Node, NodeId};
use crate::raft::Raft;
use crate::raft::Role;
use crate::raft::{Apply, RaftHandle, RaftRole};
use crate::rpc::Address;

#[derive(Debug)]
pub struct Candidate {
    pub election: Election,
    pub logger: Logger,
}

impl Raft<Candidate> {
    pub(crate) fn seek_election(mut self) -> Result<RaftHandle> {
        info!(self.role.logger, "Seeking election");
        self.state.voted_for = Some(self.id);
        self.state.current_term += 1;
        let from = self.id;
        let term = self.state.current_term;

        for _node in &self.config.nodes {
            self.send_all(Command::VoteRequest {
                term,
                candidate_id: from,
                last_term: term,
                last_index: self.state.last_applied,
            })?;
        }

        // Vote for self,
        self.apply(Command::VoteResponse {
            from,
            term,
            granted: true,
        })
    }
}

impl Role for Candidate {
    fn term(&mut self, _term: u64) {
        self.election.reset();
    }

    fn role(&self) -> RaftRole {
        RaftRole::Candidate
    }

    fn log(&self) -> &Logger {
        &self.logger
    }
}

impl Apply for Raft<Candidate> {
    fn apply(mut self, cmd: Command) -> Result<RaftHandle> {
        self.log_command(&cmd);

        match cmd {
            Command::Tick => {
                if self.needs_election() {
                    return match self.role.election.election_status() {
                        ElectionStatus::Elected => {
                            error!(self.role.logger, "This should never happen.");
                            Ok(RaftHandle::Leader(Raft::from(self)))
                        }
                        ElectionStatus::Voting => {
                            info!(self.role.logger, "Election ended with missing votes");
                            self.state.voted_for = None;
                            let raft: Raft<Follower> = Raft::from(self);
                            Ok(raft.apply(Command::Timeout)?)
                        }
                        ElectionStatus::Defeated => {
                            info!(self.role.logger, "Defeated in election.");
                            self.state.voted_for = None;
                            let raft: Raft<Follower> = Raft::from(self);
                            Ok(raft.apply(Command::Timeout)?)
                        }
                    };
                }

                Ok(RaftHandle::Candidate(self))
            }
            Command::VoteRequest {
                candidate_id,
                term: _,
                ..
            } => {
                self.send(
                    Address::Peer(candidate_id),
                    Command::VoteResponse {
                        from: self.id,
                        term: self.state.current_term,
                        granted: false,
                    },
                )?;

                Ok(RaftHandle::Candidate(self))
            }
            Command::VoteResponse { granted, from, .. } => {
                info!(self.role.logger, "Recieved vote"; "granted" => granted, "from" => from);
                self.role.election.vote(from, granted);
                match self.role.election.election_status() {
                    ElectionStatus::Elected => {
                        info!(self.role.logger, "I have been elected leader");
                        let raft = Raft::from(self);
                        raft.heartbeat()?;
                        Ok(RaftHandle::Leader(raft))
                    }
                    ElectionStatus::Voting => {
                        info!(self.role.logger, "We are still voting");
                        Ok(RaftHandle::Candidate(self))
                    }
                    ElectionStatus::Defeated => {
                        info!(self.role.logger, "I was defeated in the election");
                        self.state.voted_for = None;
                        Ok(RaftHandle::Follower(Raft::from(self)))
                    }
                }
            }
            Command::AppendEntries {
                entries: _, term, ..
            } => {
                // While waiting for votes, a candidate may receive an
                // AppendEntries RPC from another server claiming to be
                // leader. If the leader’s term (included in its RPC) is at least
                // as large as the candidate’s current term, then the candidate
                // recognizes the leader as legitimate and returns to follower
                // state.
                if term >= self.state.current_term {
                    info!(
                        self.role.logger,
                        "Received higher term, transitioning to follower"
                    );
                    let raft: Raft<Follower> = Raft::from(self);
                    //                    raft.io.append(entries)?;
                    return Ok(RaftHandle::Follower(raft));
                }

                // TODO: If the term in the RPC is smaller than the candidate’s
                // current term, then the candidate rejects the RPC and continues in candidate state.

                Ok(RaftHandle::Candidate(self))
            }
            Command::Heartbeat {
                term, leader_id: _, ..
            } => {
                if term >= self.state.current_term {
                    info!(
                        self.role.logger,
                        "Received higher term, transitioning to follower"
                    );
                    let raft: Raft<Follower> = Raft::from(self);
                    //                    raft.io.heartbeat(leader_id);
                    return Ok(RaftHandle::Follower(raft));
                }

                Ok(RaftHandle::Candidate(self))
            }
            _ => Ok(RaftHandle::Candidate(self)),
        }
    }
}

impl From<Raft<Candidate>> for Raft<Follower> {
    fn from(val: Raft<Candidate>) -> Raft<Follower> {
        Raft {
            id: val.id,
            state: val.state,
            role: Follower {
                leader_id: None,
                logger: val.logger.new(o!("role" => "follower")),
            },
            logger: val.logger,
            config: val.config,
            log: val.log,
            rpc_tx: val.rpc_tx,
            fsm_tx: val.fsm_tx,
        }
    }
}

impl From<Raft<Candidate>> for Raft<Leader> {
    fn from(val: Raft<Candidate>) -> Raft<Leader> {
        info!(val.role.logger, "Becoming the leader");

        let mut nodes: Vec<NodeId> = val.config.nodes.iter().map(|x| x.id).collect();
        nodes.push(val.id);
        let progress = ReplicationProgress::new(nodes);
        Raft {
            id: val.id,
            state: val.state,
            role: Leader {
                logger: val.logger.new(o!("role" => "leader")),
                progress,
                heartbeat_time: Instant::now(),
                heartbeat_timeout: val.config.heartbeat_timeout,
            },
            logger: val.logger,
            config: val.config,
            log: val.log,
            rpc_tx: val.rpc_tx,
            fsm_tx: val.fsm_tx,
        }
    }
}

#[cfg(test)]
mod tests {}
