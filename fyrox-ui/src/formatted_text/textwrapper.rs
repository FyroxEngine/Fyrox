use std::ops::Range;

pub trait TextWrapper {
    fn push(&mut self, c: char, advance: f32);
    fn finish(&mut self);
}

pub trait LineSink {
    fn push_line(&mut self, range: Range<usize>, width: f32);
    fn max_width(&self) -> f32;
}

fn is_newline(c: char) -> bool {
    c == '\n' || c == '\r'
}

pub struct NoWrap<S> {
    sink: S,
    start: usize,
    position: usize,
    width: f32,
}

impl<S> NoWrap<S> {
    pub fn new(sink: S) -> Self {
        NoWrap {
            sink,
            start: 0,
            position: 0,
            width: 0.0,
        }
    }
}

impl<S: LineSink> TextWrapper for NoWrap<S> {
    fn push(&mut self, c: char, advance: f32) {
        if is_newline(c) {
            self.sink.push_line(self.start..self.position, self.width);
            self.start = self.position + 1; // Next like starts after the newline, so skip ahead by one.
            self.position += 1;
            self.width = 0.0;
        } else {
            self.position += 1;
            self.width += advance;
        }
    }
    fn finish(&mut self) {
        self.sink.push_line(self.start..self.position, self.width);
    }
}

pub struct LetterWrap<S> {
    sink: S,
    start: usize,
    position: usize,
    width: f32,
}

impl<S> LetterWrap<S> {
    pub fn new(sink: S) -> Self {
        LetterWrap {
            sink,
            start: 0,
            position: 0,
            width: 0.0,
        }
    }
}

impl<S: LineSink> TextWrapper for LetterWrap<S> {
    fn push(&mut self, c: char, advance: f32) {
        if self.position != self.start && self.width + advance > self.sink.max_width() {
            self.sink.push_line(self.start..self.position, self.width);
            self.start = self.position;
            self.width = 0.0;
        }
        self.position += 1;
        self.width += advance;
        if is_newline(c) {
            self.sink.push_line(self.start..self.position, self.width);
            self.start = self.position; // Next line starts after the newline
            self.width = 0.0;
        } else {
            self.position += 1;
            self.width += advance;
        }
    }

    fn finish(&mut self) {
        self.sink.push_line(self.start..self.position, self.width);
    }
}

pub struct WordWrap<S> {
    sink: S,
    // Start of the current line
    start: usize,
    // Position of the next character
    position: usize,
    // Width of the current line
    width: f32,
    // Start of the current word
    word_start: usize,
    // Width of the current word
    word_width: f32,
}

impl<S> WordWrap<S> {
    pub fn new(sink: S) -> Self {
        WordWrap {
            sink,
            start: 0,
            position: 0,
            width: 0.0,
            word_start: 0,
            word_width: 0.0,
        }
    }
}

impl<S: LineSink> TextWrapper for WordWrap<S> {
    fn push(&mut self, c: char, advance: f32) {
        if self.position != self.start && self.width + advance > self.sink.max_width() {
            if self.start < self.word_start {
                self.sink
                    .push_line(self.start..self.word_start, self.width - self.word_width);
                // The current word becomes the current line.
                self.start = self.word_start;
                self.width = self.word_width;
            } else {
                // The current word started at or before the start of the current line,
                // so ignore the word and just wrap at the current position.
                self.sink.push_line(self.start..self.position, self.width);
                self.start = self.position;
                self.width = 0.0;
            }
        }
        self.position += 1;
        self.width += advance;
        if is_newline(c) {
            self.sink.push_line(self.start..self.position, self.width);
            self.start = self.position;
            self.width = 0.0;
            // newline is not part of a word, so move word_start ahead.
            self.word_start = self.position;
            self.word_width = 0.0;
        } else if c.is_whitespace() {
            // We are not in a word, so move word_start ahead.
            self.word_start = self.position;
            self.word_width = 0.0;
        } else {
            // We are in a word, so leave word_start alone and increase word_width.
            self.word_width += advance;
        }
    }

    fn finish(&mut self) {
        self.sink.push_line(self.start..self.position, self.width);
    }
}
