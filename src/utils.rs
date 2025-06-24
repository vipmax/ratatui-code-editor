pub fn get_lang(filename: &str) -> String {

    let extension = std::path::Path::new(filename).extension()
        .and_then(|ext| ext.to_str()).unwrap_or("");

    match extension {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "javascript",
        "tsx" => "typescript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp"  => "cpp",
        "c" => "c",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        _ => "unknown",
    }
    .to_string()
}