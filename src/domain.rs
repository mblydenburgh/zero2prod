use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName
}

pub struct SubscriberName(String);


impl SubscriberName {
    pub fn parse(s: String) -> SubscriberName {
        let is_empty_or_whitespace = s.trim().is_empty();
        // Note: grapheme is is a special char that appears as 1 unicode char but is actualy 2
        // `graphmemes` returns an interator over the graphemes in the input s, true specifies
        // that we want to use the extended grapheme definitions set
        let is_too_long = s.graphemes(true).count() > 256;
        let invalid_chars = ['/', '(', ')', '[', ']', '{', '}', '"', '<', '>', '\\'];
        let contains_invalid_chars = s
            .chars()
            .any(|c| invalid_chars.contains(&c));
        if is_empty_or_whitespace || is_too_long || contains_invalid_chars {
            panic!("{} is not a valid name", s)
        } else {
            Self(s)
        }
    }
    pub fn inner(self) -> String {
        self.0
    }
    pub fn inner_mut(&mut self) -> &mut str {
        &mut self.0
    }
    pub fn inner_ref(&self) -> &str {
        &self.0
    }
}
