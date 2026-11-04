use std::time::Duration;

pub fn timecode(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_appear_only_when_needed() {
        assert_eq!(timecode(Duration::from_secs(5)), "0:05");
        assert_eq!(timecode(Duration::from_secs(754)), "12:34");
        assert_eq!(timecode(Duration::from_secs(3723)), "1:02:03");
    }
}
