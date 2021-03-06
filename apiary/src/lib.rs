#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

pub mod app;
pub mod error;
mod input;
pub mod reader_threads;
pub mod rendering;
mod scenes;
pub mod time;
