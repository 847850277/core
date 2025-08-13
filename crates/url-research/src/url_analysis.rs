use std::collections::HashMap;
use url::Url;
use serde::{Serialize, Deserialize};

/// 分析不同类型的 URL 字符串
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlAnalysis {
    pub input: String,
    pub is_valid: bool,
    pub scheme: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub error: Option<String>,
}

impl UrlAnalysis {
    pub fn analyze(input: &str) -> Self {
        match Url::parse(input) {
            Ok(url) => Self {
                input: input.to_string(),
                is_valid: true,
                scheme: Some(url.scheme().to_string()),
                host: url.host_str().map(|h| h.to_string()),
                path: Some(url.path().to_string()),
                query: url.query().map(|q| q.to_string()),
                fragment: url.fragment().map(|f| f.to_string()),
                error: None,
            },
            Err(e) => Self {
                input: input.to_string(),
                is_valid: false,
                scheme: None,
                host: None,
                path: None,
                query: None,
                fragment: None,
                error: Some(e.to_string()),
            },
        }
    }
}

/// URL 研究器 - 系统性地测试各种 URL 格式
pub struct UrlResearcher {
    test_cases: Vec<&'static str>,
}

impl UrlResearcher {
    pub fn new() -> Self {
        Self {
            test_cases: vec![
                // 标准 HTTP/HTTPS URLs
                "http://example.com",
                "https://github.com/user/repo/file.wasm",
                "http://localhost:8080/app.wasm",
                
                // File URLs
                "file:///absolute/path/to/file.wasm",
                "file://localhost/path/to/file.wasm",
                "file:///C:/Windows/path/file.wasm",
                
                // 自定义 schemes
                "local://application",
                "app://local-file",
                "custom://identifier",
                "dev://local-development",
                
                // 相对路径 (应该失败)
                "relative/path/file.wasm",
                "file.wasm",
                "./local/file.wasm",
                "../parent/file.wasm",
                
                // 特殊情况
                "ftp://ftp.example.com/file.wasm",
                "data:application/wasm;base64,SGVsbG8gV29ybGQ=",
                "blob:http://example.com/550e8400-e29b-41d4-a716-446655440000",
                
                // 边界情况
                "",
                "/",
                "://invalid",
                "http://",
                "scheme:",
                
                // 带有特殊字符的路径
                "file:///path/with spaces/file.wasm",
                "http://example.com/path/with%20encoded%20spaces",
                "custom://app/特殊字符/文件.wasm",
            ]
        }
    }
    
    pub fn run_analysis(&self) -> Vec<UrlAnalysis> {
        self.test_cases
            .iter()
            .map(|&test_case| UrlAnalysis::analyze(test_case))
            .collect()
    }
    
    pub fn categorize_results(&self) -> HashMap<String, Vec<UrlAnalysis>> {
        let results = self.run_analysis();
        let mut categories = HashMap::new();
        
        for analysis in results {
            let category = if !analysis.is_valid {
                "Invalid".to_string()
            } else if let Some(ref scheme) = analysis.scheme {
                match scheme.as_str() {
                    "http" | "https" => "Web URLs".to_string(),
                    "file" => "File URLs".to_string(),
                    "ftp" => "FTP URLs".to_string(),
                    "data" => "Data URLs".to_string(),
                    "blob" => "Blob URLs".to_string(),
                    _ => "Custom Schemes".to_string(),
                }
            } else {
                "Unknown".to_string()
            };
            
            categories.entry(category).or_insert_with(Vec::new).push(analysis);
        }
        
        categories
    }
    
    pub fn print_detailed_report(&self) {
        let categories = self.categorize_results();
        
        println!("=== URL 类型详细分析报告 ===\n");
        
        for (category, analyses) in categories {
            println!("📁 {}", category);
            println!("{}", "─".repeat(50));
            
            for analysis in analyses {
                if analysis.is_valid {
                    println!("✅ Input: '{}'", analysis.input);
                    println!("   Scheme: {}", analysis.scheme.as_deref().unwrap_or("N/A"));
                    println!("   Host: {}", analysis.host.as_deref().unwrap_or("N/A"));
                    println!("   Path: {}", analysis.path.as_deref().unwrap_or("N/A"));
                    if let Some(ref query) = analysis.query {
                        println!("   Query: {}", query);
                    }
                    if let Some(ref fragment) = analysis.fragment {
                        println!("   Fragment: {}", fragment);
                    }
                } else {
                    println!("❌ Input: '{}'", analysis.input);
                    println!("   Error: {}", analysis.error.as_deref().unwrap_or("Unknown error"));
                }
                println!();
            }
        }
    }
}

impl Default for UrlResearcher {
    fn default() -> Self {
        Self::new()
    }
}

/// 测试 URL 构造的不同方法
pub struct UrlConstructionResearcher;

impl UrlConstructionResearcher {
    pub fn test_file_path_conversion() {
        println!("=== 文件路径到 URL 转换测试 ===\n");
        
        let test_paths = vec![
            "/absolute/path/to/file.wasm",
            "/Users/user/app.wasm",
            "/tmp/temporary_app.wasm",
            "C:\\Windows\\app.wasm", // Windows 路径
        ];
        
        for path in test_paths {
            println!("Path: {}", path);
            
            // 方法1: Url::from_file_path
            match Url::from_file_path(path) {
                Ok(url) => println!("  from_file_path(): {}", url),
                Err(_) => println!("  from_file_path(): Failed"),
            }
            
            // 方法2: 手动构造
            let manual_url = format!("file://{}", path);
            match Url::parse(&manual_url) {
                Ok(url) => println!("  manual parse: {}", url),
                Err(e) => println!("  manual parse: Error - {}", e),
            }
            
            println!();
        }
    }
    
    pub fn test_custom_schemes() {
        println!("=== 自定义 Scheme 测试 ===\n");
        
        let custom_urls = vec![
            "local://application",
            "app://development", 
            "dev://local-file",
            "custom://identifier/with/path",
            "internal://app/v1.0.0",
            "temp://session-12345",
        ];
        
        for url_str in custom_urls {
            match Url::parse(url_str) {
                Ok(url) => {
                    println!("✅ {}", url_str);
                    println!("   Scheme: {}", url.scheme());
                    println!("   Host: {}", url.host_str().unwrap_or("N/A"));
                    println!("   Path: {}", url.path());
                },
                Err(e) => println!("❌ {} - Error: {}", url_str, e),
            }
            println!();
        }
    }
}
