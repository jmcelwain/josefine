mod raft;
mod election;
mod follower;
mod candidate;
mod leader;
mod config;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}