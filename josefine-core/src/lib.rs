extern crate slog;
extern crate slog_async;
extern crate slog_term;

mod entry;
mod index;
mod log;
mod partition;
mod segment;
mod server;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
