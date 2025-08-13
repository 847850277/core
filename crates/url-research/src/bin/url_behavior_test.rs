use url_research::UrlResearcher;

fn main() -> eyre::Result<()> {
    println!("🔬 URL 类型行为详细研究\n");
    
    let researcher = UrlResearcher::new();
    
    // 打印详细报告
    researcher.print_detailed_report();
    
    // 测试文件路径转换
    url_research::UrlConstructionResearcher::test_file_path_conversion();
    
    // 测试自定义 schemes
    url_research::UrlConstructionResearcher::test_custom_schemes();
    
    // 生成 JSON 报告
    println!("=== JSON 格式分析结果 ===\n");
    let categories = researcher.categorize_results();
    for (category, analyses) in categories {
        println!("Category: {}", category);
        for analysis in analyses {
            if let Ok(json) = serde_json::to_string_pretty(&analysis) {
                println!("{}", json);
            }
        }
        println!();
    }
    
    Ok(())
}
