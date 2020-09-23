use std::io::Read;

use anyhow::{anyhow, ensure};

fn do_update_check() -> anyhow::Result<()> {
    let repository = env!("CARGO_PKG_REPOSITORY");
    ensure!(!repository.is_empty(), "repository not set");
    let mut client = git_transport::connect(
        repository.as_bytes(),
        git_transport::Protocol::V2,
    )?;
    let handshake = client.handshake(git_transport::Service::UploadPack)?;
    let mut refs = handshake.refs
        .ok_or_else(|| anyhow!("no refs returned from server"))?;

    let mut ref_data = String::new();
    refs.read_to_string(&mut ref_data)?;

    let ref_lines = ref_data.split('\n');
    let ref_descs = ref_lines.map(|line| line.split_whitespace());
    let ref_names = ref_descs.filter_map(|line_pieces| line_pieces.skip(1).next());

    let version_tags = ref_names.filter_map(|ref_name| ref_name.strip_prefix("refs/tags/v"));
    let versions = version_tags.filter_map(|version| version.parse::<semver::Version>().ok());

    let latest_version = versions.max()
        .ok_or_else(|| anyhow!("no published versions"))?;
    let current_version = env!("CARGO_PKG_VERSION").parse::<semver::Version>()?;

    if current_version < latest_version {
        warn!("You're using version {} of Crotch-Stim: Get Off, but the latest version is {}.", current_version, latest_version);
        warn!("Grab the update at {}", env!("CARGO_PKG_HOMEPAGE"));
    }

    Ok(())
}

pub fn check_for_updates() {
    if let Err(err) = do_update_check() {
        warn!("update check failed: {}", err);
    }
}
