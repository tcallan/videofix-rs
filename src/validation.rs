use crate::metadata;

use super::FormatSpec;
use super::Formats;

#[derive(Debug)]
pub(crate) struct FormatValidation {
    pub(crate) audio_okay: bool,
    pub(crate) video_okay: bool,
    pub(crate) container_okay: bool,
}

impl FormatValidation {
    pub(crate) fn is_valid(&self) -> bool {
        self.audio_okay && self.video_okay && self.container_okay
    }
}

pub(crate) fn validate_format(
    file: &metadata::FileMetadata,
    format: &FormatSpec,
) -> FormatValidation {
    match format {
        FormatSpec::Allow(f) => validate_format_selector(file, f, &allow),
        FormatSpec::Reject(f) => validate_format_selector(file, f, &deny),
    }
}

fn allow(format: &[String], value: &String) -> bool {
    format.contains(value)
}

fn deny(format: &[String], value: &String) -> bool {
    !allow(format, value)
}

fn validate_format_selector(
    file: &metadata::FileMetadata,
    formats: &Formats,
    selector: &dyn Fn(&[String], &String) -> bool,
) -> FormatValidation {
    let audio_okay = selector(&formats.audio, &file.audio.codec);
    let video_okay = selector(&formats.video, &file.video.codec);
    let container_okay = selector(&formats.container, &file.container);

    FormatValidation {
        audio_okay,
        video_okay,
        container_okay,
    }
}

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;
    use crate::metadata::{AudioMetadata, FileMetadata, VideoMetadata};

    fn str_vec(v: Vec<&str>) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect_vec()
    }

    fn mk_metadata(container: &str, vcodec: &str, acodec: &str) -> FileMetadata {
        FileMetadata {
            container: container.to_string(),
            duration: None,
            video: VideoMetadata {
                index: 0,
                codec: vcodec.to_string(),
            },
            audio: AudioMetadata {
                index: 1,
                codec: acodec.to_string(),
                channels: 2,
            },
        }
    }

    #[test]
    fn format_validation_allow_valid() {
        let format = FormatSpec::Allow(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mp4", "h265", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_container() {
        let format = FormatSpec::Allow(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mkv", "h265", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_video() {
        let format = FormatSpec::Allow(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mp4", "avi", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_allow_invalid_audio() {
        let format = FormatSpec::Allow(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mp4", "h265", "flac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_valid() {
        let format = FormatSpec::Reject(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mkv", "mp4", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_container() {
        let format = FormatSpec::Reject(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("avi", "mp4", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_video() {
        let format = FormatSpec::Reject(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mkv", "h264", "aac");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }

    #[test]
    fn format_validation_reject_invalid_audio() {
        let format = FormatSpec::Reject(Formats {
            audio: str_vec(vec!["mp3", "wav"]),
            video: str_vec(vec!["h264", "h265"]),
            container: str_vec(vec!["avi", "mp4"]),
        });
        let metadata = mk_metadata("mkv", "mp4", "mp3");

        let validation = validate_format(&metadata, &format);
        assert!(!validation.is_valid());
    }
}
