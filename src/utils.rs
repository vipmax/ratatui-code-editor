pub fn get_lang(filename: &str) -> String {

    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension {
        "rs" => "rust",
        "js" | "jsx"  => "javascript",
        "ts" | "tsx"=> "typescript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp"  => "cpp",
        "c" => "c",
        "cs" => "c_sharp",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" | "bash" => "shell",
        _ => "unknown",
    }
    .to_string()
}

pub fn indent(lang: &str) -> String {
    match lang {
        "rust" |"python" | "php" | "toml" | "c"  | "cpp" |
        "zig" | "kotlin" | "erlang" | "html" | "sql" => {
            "    ".to_string()
        },
        "go" | "c_sharp" => {
            "\t".to_string()
        },

        _ => "  ".to_string(),
    }
}

pub fn rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}