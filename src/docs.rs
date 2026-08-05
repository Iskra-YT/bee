use anyhow::Result;

pub fn update_docs() -> Result<()> {
    let files: Vec<(&str, &str)> = vec![
        ("bee/README.md", crate::init::BEE_README),
        ("bee/AGENTS.md", crate::init::AGENTS_MD),
        ("bee/CLAUDE.md", crate::init::CLAUDE_MD),
    ];

    for (path, content) in files {
        crate::file::write_file_content(&String::from(path), &content.to_string())?;
        println!("Updated {}", path);
    }

    println!("Documentation updated");
    Ok(())
}
