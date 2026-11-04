use std::path::PathBuf;

pub const URI_LIST_MIME: &str = "text/uri-list";

/// The local paths in a `text/uri-list` (RFC 2483), skipping comments and remote URIs.
pub fn file_paths(uri_list: &str) -> impl Iterator<Item = PathBuf> {
    uri_list
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.strip_prefix("file://"))
        .filter_map(|authority_and_path| {
            let path = &authority_and_path[authority_and_path.find('/')?..];
            percent_decode(path).map(PathBuf::from)
        })
}

fn percent_decode(encoded: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut rest = encoded.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let hex = std::str::from_utf8(tail.get(..2)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_uris_become_decoded_paths() {
        let list = "# dragged from Files\r\nfile:///home/me/My%20Shot.png\r\nfile://host/tmp/a.pdf\r\nhttps://example.com/x.png\r\n";
        let paths: Vec<_> = file_paths(list).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("/home/me/My Shot.png"),
                PathBuf::from("/tmp/a.pdf")
            ]
        );
    }

    #[test]
    fn a_truncated_escape_is_rejected() {
        assert_eq!(file_paths("file:///tmp/bad%2").count(), 0);
    }
}
