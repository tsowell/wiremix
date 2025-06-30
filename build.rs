use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Put "git describe" output (ex. "v0.5.0-2-gbc03f8a-dirty") in
    // VERGEN_GIT_DESCRIBE environment variable for use in --version output.
    let describe = Git2Builder::default()
        .describe(true, true, Some("v[0-9]*.[0-9]*.[0-9]*"))
        .build()?;
    Emitter::default().add_instructions(&describe)?.emit()?;

    Ok(())
}
