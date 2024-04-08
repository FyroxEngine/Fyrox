use base64::engine::general_purpose::URL_SAFE_NO_PAD as Base64Engine;
use base64::engine::Engine as _;

/// A breakdown of the data contained within a URI.
#[derive(Copy, Clone, Debug)]
pub struct Uri<'a> {
    /// Reference to the full URI str.
    pub original: &'a str,
    /// The recognized scheme type, if any.
    pub scheme: Scheme,
    /// The text that was used to identify the scheme, preceeding the first ':', if any.
    /// For example, if the URI were <https://www.somewebsite.org/books/RestInPractice.pdf>
    /// then  `scheme_name` would be the "https" from the beginning.
    pub scheme_name: Option<&'a str>,
    /// Everything that follows the initial ':', or the whole original str if there is no ':'.
    /// If the URI were <https://www.somewebsite.org/books/RestInPractice.pdf>
    /// then `after_scheme` would be <www.somewebsite.org/books/RestInPractice.pdf>.
    pub after_scheme: &'a str,
    /// If the scheme is "data" then `data_type` is the slice between the first ':' and the first ','.
    /// For example, if the URI were ""data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAF", then
    /// `data_type` would be "image/png;base64". If there is no ',' then `data_type` is the same as
    /// `after_scheme`. If the scheme is not "data" then `data_type` is None.
    pub data_type: Option<&'a str>,
    /// If the scheme is "data" then `data` is everything following the first ','.
    /// `data_type` is None if the scheme is not "data" or if there is no ','.
    pub data: Option<&'a str>,
}

/// One of the scheme types that the parser can recognize.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Scheme {
    /// A file URI indicates that this is an absolute path to a file.
    /// This method of identifying files is not supported.
    File,
    /// A data URI contains image or model data encoded in base64.
    Data,
    /// There was no ':' to identify a scheme name, which means that this URI
    /// will be interpreted as a relative file path and the file system will
    /// be searched for the named asset.
    None,
    /// A scheme name was specified using a ':', but it was not among these
    /// expected schemes.
    Other,
}

/// Turn a URI str slice into a Uri structure containing the parts of the URI.
pub fn parse_uri(source: &str) -> Uri {
    let (scheme_name, after_scheme): (Option<&str>, &str) = {
        let mut scheme_it = source.splitn(2, ':');
        let first = scheme_it.next();
        let second = scheme_it.next();
        if let Some(rest) = second {
            (first, rest)
        } else {
            (None, first.unwrap())
        }
    };
    if scheme_name == Some("data") {
        let mut it = after_scheme.splitn(2, ',');
        Uri {
            original: source,
            scheme: Scheme::Data,
            scheme_name,
            after_scheme,
            data_type: it.next(),
            data: it.next(),
        }
    } else if let Some(name) = scheme_name {
        let scheme = match name {
            "file" => Scheme::File,
            _ => Scheme::Other,
        };
        Uri {
            original: source,
            scheme,
            scheme_name,
            after_scheme,
            data_type: None,
            data: None,
        }
    } else {
        Uri {
            original: source,
            scheme: Scheme::None,
            scheme_name,
            after_scheme,
            data_type: None,
            data: None,
        }
    }
}

pub fn decode_base64(source: &str) -> Result<Vec<u8>, base64::DecodeError> {
    Base64Engine.decode(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split() {
        let r: Vec<_> = "abc:def".splitn(2, ':').collect();
        assert!(r == vec!["abc", "def"], "{:?}", r);
    }
    #[test]
    fn simple() {
        let u: Uri = parse_uri("Random stuff");
        assert!(
            matches!(
                u,
                Uri {
                    original: "Random stuff",
                    scheme: Scheme::None,
                    scheme_name: None,
                    after_scheme: "Random stuff",
                    data_type: None,
                    data: None,
                }
            ),
            "{:?}",
            u
        )
    }
    #[test]
    fn file() {
        let u: Uri = parse_uri("file:filename.test");
        assert!(
            matches!(
                u,
                Uri {
                    original: "file:filename.test",
                    scheme: Scheme::File,
                    scheme_name: Some("file"),
                    after_scheme: "filename.test",
                    data_type: None,
                    data: None,
                }
            ),
            "{:?}",
            u
        )
    }
    #[test]
    fn other() {
        let u: Uri = parse_uri("what:Stuff");
        assert!(
            matches!(
                u,
                Uri {
                    original: "what:Stuff",
                    scheme: Scheme::Other,
                    scheme_name: Some("what"),
                    after_scheme: "Stuff",
                    data_type: None,
                    data: None,
                }
            ),
            "{:?}",
            u
        )
    }
    #[test]
    fn data() {
        let u: Uri = parse_uri("data:type,ABCDEFG");
        assert!(
            matches!(
                u,
                Uri {
                    original: "data:type,ABCDEFG",
                    scheme: Scheme::Data,
                    scheme_name: Some("data"),
                    after_scheme: "type,ABCDEFG",
                    data_type: Some("type"),
                    data: Some("ABCDEFG"),
                }
            ),
            "{:?}",
            u
        )
    }
}
