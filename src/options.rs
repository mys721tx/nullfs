//! Parsing of `-o` mount options.

use fuser::{Config, MountOption, SessionACL};

/// Builds a session configuration from comma-separated `-o` mount options.
pub fn parse_options<'a>(options: impl Iterator<Item = &'a str>) -> Config {
    let mut config = Config::default();

    for option in options.flat_map(|x| x.split(',')).filter(|x| !x.is_empty()) {
        match option {
            "allow_other" => config.acl = SessionACL::All,
            "allow_root" => config.acl = SessionACL::RootAndOwner,
            _ => config.mount_options.push(parse_mount_option(option)),
        }
    }

    config
}

/// Maps a single mount option to its [`MountOption`]; unknown options are
/// passed through as [`MountOption::CUSTOM`].
pub fn parse_mount_option(option: &str) -> MountOption {
    match option {
        "auto_unmount" => MountOption::AutoUnmount,
        "default_permissions" => MountOption::DefaultPermissions,
        "dev" => MountOption::Dev,
        "nodev" => MountOption::NoDev,
        "suid" => MountOption::Suid,
        "nosuid" => MountOption::NoSuid,
        "ro" => MountOption::RO,
        "rw" => MountOption::RW,
        "exec" => MountOption::Exec,
        "noexec" => MountOption::NoExec,
        "atime" => MountOption::Atime,
        "noatime" => MountOption::NoAtime,
        "dirsync" => MountOption::DirSync,
        "sync" => MountOption::Sync,
        "async" => MountOption::Async,
        _ => match option.split_once('=') {
            Some(("fsname", name)) => MountOption::FSName(name.into()),
            Some(("subtype", name)) => MountOption::Subtype(name.into()),
            _ => MountOption::CUSTOM(option.into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_separated_options() {
        let config =
            parse_options(["ro,nosuid", "fsname=null", "allow_other", "foo=bar"].into_iter());

        assert_eq!(
            config.mount_options,
            [
                MountOption::RO,
                MountOption::NoSuid,
                MountOption::FSName("null".into()),
                MountOption::CUSTOM("foo=bar".into()),
            ]
        );
        assert_eq!(config.acl, SessionACL::All);
    }

    #[test]
    fn no_options_yields_default_config() {
        assert_eq!(parse_options(std::iter::empty()), Config::default());
    }

    #[test]
    fn empty_segments_are_ignored() {
        assert_eq!(
            parse_options(["", ",", ",,"].into_iter()),
            Config::default()
        );
        assert_eq!(
            parse_options([",ro,,rw,"].into_iter()).mount_options,
            [MountOption::RO, MountOption::RW]
        );
    }

    #[test]
    fn last_acl_option_wins() {
        let config = parse_options(["allow_other,allow_root"].into_iter());
        assert_eq!(config.acl, SessionACL::RootAndOwner);
        assert!(config.mount_options.is_empty());

        let config = parse_options(["allow_root", "allow_other"].into_iter());
        assert_eq!(config.acl, SessionACL::All);
    }

    #[test]
    fn every_keyword_option_is_recognised() {
        let cases = [
            ("auto_unmount", MountOption::AutoUnmount),
            ("default_permissions", MountOption::DefaultPermissions),
            ("dev", MountOption::Dev),
            ("nodev", MountOption::NoDev),
            ("suid", MountOption::Suid),
            ("nosuid", MountOption::NoSuid),
            ("ro", MountOption::RO),
            ("rw", MountOption::RW),
            ("exec", MountOption::Exec),
            ("noexec", MountOption::NoExec),
            ("atime", MountOption::Atime),
            ("noatime", MountOption::NoAtime),
            ("dirsync", MountOption::DirSync),
            ("sync", MountOption::Sync),
            ("async", MountOption::Async),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_mount_option(input), expected, "option {input:?}");
        }
    }

    #[test]
    fn key_value_options() {
        assert_eq!(
            parse_mount_option("fsname=nullfs"),
            MountOption::FSName("nullfs".into())
        );
        assert_eq!(
            parse_mount_option("subtype=null"),
            MountOption::Subtype("null".into())
        );
        // Only the first '=' separates key and value.
        assert_eq!(
            parse_mount_option("fsname=a=b"),
            MountOption::FSName("a=b".into())
        );
        assert_eq!(
            parse_mount_option("fsname="),
            MountOption::FSName(String::new())
        );
    }

    #[test]
    fn unknown_options_pass_through() {
        for input in ["foo", "foo=bar", "RO", "=x", "max_read=4096"] {
            assert_eq!(
                parse_mount_option(input),
                MountOption::CUSTOM(input.into()),
                "option {input:?}"
            );
        }
    }
}
