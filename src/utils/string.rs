pub trait StringExt {
    fn count_words(&self, word: &str) -> u32;
}

impl StringExt for str {
    fn count_words(&self, word: &str) -> u32 {
        self.split(word).count() as u32
    }
}
