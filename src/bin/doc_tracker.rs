// Documentation Change Tracking CLI Tool for ShrivenQ
// Implements automatic reference tracking and propagation system

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::fs;
use std::error::Error;
use std::fmt;

#[derive(Parser)]
#[command(name = "doc-tracker")]
#[command(about = "ShrivenQ Documentation Change Tracking System")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan documentation and build reference graph
    Scan {
        /// Documentation root directory
        #[arg(short, long, default_value = "docs")]
        docs_path: PathBuf,
        /// Output file for reference graph
        #[arg(short, long, default_value = "docs/.doc-graph.json")]
        output: PathBuf,
        /// Include source code references
        #[arg(long)]
        include_source: bool,
    },
    /// Watch for changes and auto-propagate (future implementation)
    Watch {
        /// Documentation root directory  
        #[arg(short, long, default_value = "docs")]
        docs_path: PathBuf,
        /// Dry run mode (show what would be changed)
        #[arg(long)]
        dry_run: bool,
        /// Auto-apply threshold (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        auto_threshold: f32,
    },
    /// Validate documentation consistency
    Validate {
        /// Documentation root directory
        #[arg(short, long, default_value = "docs")]
        docs_path: PathBuf,
        /// Fix issues automatically
        #[arg(long)]
        fix: bool,
        /// Show detailed output
        #[arg(long)]
        verbose: bool,
    },
    /// Generate documentation metrics
    Metrics {
        /// Documentation root directory
        #[arg(short, long, default_value = "docs")]
        docs_path: PathBuf,
        /// Output format (json, markdown)
        #[arg(long, default_value = "markdown")]
        format: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocReference {
    pub source_file: PathBuf,
    pub target_path: String,
    pub reference_type: ReferenceType,
    pub line_number: usize,
    pub context: String,
    pub anchor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum ReferenceType {
    DirectLink,           // [link](path.md)
    CodeReference,        // See src/core/memory.rs
    ConfigValue,          // References to config values
    FunctionName,         // Function/struct/enum references
    PerformanceMetric,    // Performance targets/metrics
    BuildScript,          // Build script references
    FeatureFlag,          // Feature flag documentation
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentationGraph {
    pub references: Vec<DocReference>,
    pub files: HashMap<PathBuf, DocMetadata>,
    pub broken_links: Vec<DocReference>,
    pub metrics: GraphMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocMetadata {
    pub title: String,
    pub last_modified: String,
    pub word_count: usize,
    pub reference_count: usize,
    pub checksum: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GraphMetrics {
    pub total_files: usize,
    pub total_references: usize,
    pub broken_references: usize,
    pub most_referenced_files: Vec<(PathBuf, usize)>,
    pub reference_type_counts: HashMap<ReferenceType, usize>,
}

#[derive(Debug)]
pub struct DocError {
    message: String,
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Documentation Error: {}", self.message)
    }
}

impl Error for DocError {}

impl From<std::io::Error> for DocError {
    fn from(err: std::io::Error) -> Self {
        DocError { message: err.to_string() }
    }
}

impl From<serde_json::Error> for DocError {
    fn from(err: serde_json::Error) -> Self {
        DocError { message: err.to_string() }
    }
}

pub struct DocumentationScanner {
    #[allow(dead_code)]
    include_source: bool,
}

impl DocumentationScanner {
    pub fn new(include_source: bool) -> Self {
        Self { include_source }
    }

    pub fn scan_directory(&self, docs_path: &Path) -> Result<DocumentationGraph, DocError> {
        let mut references = Vec::new();
        let mut files = HashMap::new();
        let mut broken_links = Vec::new();

        println!("🔍 Scanning documentation in {}", docs_path.display());

        // Find all markdown files
        let md_files = self.find_markdown_files(docs_path)?;
        
        for file_path in md_files {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| DocError { message: format!("Failed to read {}: {}", file_path.display(), e) })?;
            
            // Extract metadata
            let metadata = self.extract_metadata(&file_path, &content);
            files.insert(file_path.clone(), metadata);

            // Extract references
            let file_refs = self.extract_references(&file_path, &content, docs_path);
            
            // Validate references and categorize
            for reference in file_refs {
                if self.validate_reference(&reference, docs_path) {
                    references.push(reference);
                } else {
                    broken_links.push(reference);
                }
            }
        }

        // Calculate metrics
        let metrics = self.calculate_metrics(&references, &files, &broken_links);

        Ok(DocumentationGraph {
            references,
            files,
            broken_links,
            metrics,
        })
    }

    fn find_markdown_files(&self, dir: &Path) -> Result<Vec<PathBuf>, DocError> {
        let mut files = Vec::new();
        
        fn visit_dir(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        // Skip hidden directories
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if !name.starts_with('.') {
                                visit_dir(&path, files)?;
                            }
                        }
                    } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext == "md" {
                            files.push(path);
                        }
                    }
                }
            }
            Ok(())
        }

        visit_dir(dir, &mut files)?;
        Ok(files)
    }

    fn extract_references(&self, file_path: &Path, content: &str, _docs_root: &Path) -> Vec<DocReference> {
        let mut references = Vec::new();
        
        // Markdown links: [text](path)
        let link_regex = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        
        for (line_num, line) in content.lines().enumerate() {
            for cap in link_regex.captures_iter(line) {
                let _link_text = cap.get(1).unwrap().as_str();
                let link_path = cap.get(2).unwrap().as_str();
                
                // Skip external URLs
                if link_path.starts_with("http://") || link_path.starts_with("https://") {
                    continue;
                }
                
                references.push(DocReference {
                    source_file: file_path.to_path_buf(),
                    target_path: link_path.to_string(),
                    reference_type: ReferenceType::DirectLink,
                    line_number: line_num + 1,
                    context: line.to_string(),
                    anchor: self.extract_anchor(link_path),
                });
            }

            // Code references: `src/core/memory.rs:123`
            let code_ref_regex = regex::Regex::new(r"`([^`]*\.rs(?::\d+)?)`").unwrap();
            for cap in code_ref_regex.captures_iter(line) {
                let code_path = cap.get(1).unwrap().as_str();
                
                references.push(DocReference {
                    source_file: file_path.to_path_buf(),
                    target_path: code_path.to_string(),
                    reference_type: ReferenceType::CodeReference,
                    line_number: line_num + 1,
                    context: line.to_string(),
                    anchor: None,
                });
            }

            // Performance metrics: "< 100μs", "1000+ orders/second"
            let perf_regex = regex::Regex::new(r"([<>≤≥]?\s*\d+(?:\.\d+)?[+]?)\s*(μs|ms|ns|orders?/second|messages?/second)").unwrap();
            for cap in perf_regex.captures_iter(line) {
                let value = cap.get(1).unwrap().as_str();
                let unit = cap.get(2).unwrap().as_str();
                
                references.push(DocReference {
                    source_file: file_path.to_path_buf(),
                    target_path: format!("performance-target:{}{}", value.trim(), unit),
                    reference_type: ReferenceType::PerformanceMetric,
                    line_number: line_num + 1,
                    context: line.to_string(),
                    anchor: Some(format!("{}{}", value.trim(), unit)),
                });
            }

            // Feature flags: `hft-unsafe`, `gpu-acceleration`
            let feature_regex = regex::Regex::new(r"`([a-z-]+(?:-[a-z]+)*)`").unwrap();
            for cap in feature_regex.captures_iter(line) {
                let feature = cap.get(1).unwrap().as_str();
                
                // Common feature flag patterns
                if feature.contains("-") && (feature.contains("unsafe") || feature.contains("gpu") || feature.contains("integration")) {
                    references.push(DocReference {
                        source_file: file_path.to_path_buf(),
                        target_path: format!("feature-flag:{}", feature),
                        reference_type: ReferenceType::FeatureFlag,
                        line_number: line_num + 1,
                        context: line.to_string(),
                        anchor: Some(feature.to_string()),
                    });
                }
            }
        }

        references
    }

    fn extract_anchor(&self, link_path: &str) -> Option<String> {
        if let Some(pos) = link_path.find('#') {
            Some(link_path[pos + 1..].to_string())
        } else {
            None
        }
    }

    fn extract_metadata(&self, _file_path: &Path, content: &str) -> DocMetadata {
        let title = self.extract_title(content);
        let word_count = content.split_whitespace().count();
        let reference_count = content.matches("](").count(); // Quick markdown link count
        let checksum = self.calculate_checksum(content);
        
        DocMetadata {
            title,
            last_modified: chrono::Utc::now().to_rfc3339(),
            word_count,
            reference_count,
            checksum,
        }
    }

    fn extract_title(&self, content: &str) -> String {
        // Look for the first # heading
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("# ") {
                return line[2..].trim().to_string();
            }
        }
        "Untitled".to_string()
    }

    fn calculate_checksum(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn validate_reference(&self, reference: &DocReference, docs_root: &Path) -> bool {
        match reference.reference_type {
            ReferenceType::DirectLink => {
                let target_path = if reference.target_path.starts_with("/") {
                    docs_root.join(&reference.target_path[1..])
                } else {
                    reference.source_file.parent().unwrap().join(&reference.target_path)
                };
                
                // Remove anchor for file existence check
                let file_path = if let Some(pos) = reference.target_path.find('#') {
                    let path_without_anchor = &reference.target_path[..pos];
                    if path_without_anchor.starts_with("/") {
                        docs_root.join(&path_without_anchor[1..])
                    } else {
                        reference.source_file.parent().unwrap().join(path_without_anchor)
                    }
                } else {
                    target_path
                };
                
                file_path.exists()
            }
            ReferenceType::CodeReference => {
                // For code references, check if the file exists in src/
                let code_path = reference.target_path.split(':').next().unwrap();
                let src_path = docs_root.parent().unwrap_or(docs_root).join(code_path);
                src_path.exists()
            }
            _ => true, // For other types, assume valid for now
        }
    }

    fn calculate_metrics(&self, references: &[DocReference], files: &HashMap<PathBuf, DocMetadata>, broken_links: &[DocReference]) -> GraphMetrics {
        let mut reference_type_counts = HashMap::new();
        let mut file_reference_counts: HashMap<String, usize> = HashMap::new();

        for reference in references {
            *reference_type_counts.entry(reference.reference_type.clone()).or_insert(0) += 1;
            *file_reference_counts.entry(reference.target_path.clone()).or_insert(0) += 1;
        }

        let mut most_referenced: Vec<(PathBuf, usize)> = file_reference_counts
            .into_iter()
            .map(|(path, count)| (PathBuf::from(path), count))
            .collect();
        most_referenced.sort_by(|a, b| b.1.cmp(&a.1));
        most_referenced.truncate(10);

        GraphMetrics {
            total_files: files.len(),
            total_references: references.len(),
            broken_references: broken_links.len(),
            most_referenced_files: most_referenced,
            reference_type_counts,
        }
    }
}

pub struct DocumentationValidator;

impl DocumentationValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, docs_path: &Path, verbose: bool) -> Result<Vec<ValidationIssue>, DocError> {
        let scanner = DocumentationScanner::new(false);
        let graph = scanner.scan_directory(docs_path)?;
        
        let mut issues = Vec::new();

        // Check for broken links
        for broken_link in &graph.broken_links {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                file: broken_link.source_file.clone(),
                line: broken_link.line_number,
                description: format!("Broken link: {}", broken_link.target_path),
                suggestion: None,
            });
        }

        // Check for orphaned files (files not referenced by any other file)
        let referenced_files: HashSet<PathBuf> = graph.references
            .iter()
            .filter_map(|r| {
                if r.reference_type == ReferenceType::DirectLink {
                    Some(PathBuf::from(&r.target_path))
                } else {
                    None
                }
            })
            .collect();

        for (file_path, _) in &graph.files {
            let relative_path = file_path.strip_prefix(docs_path).unwrap_or(file_path);
            if !referenced_files.contains(relative_path) && relative_path.file_name().unwrap() != "README.md" {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    file: file_path.clone(),
                    line: 0,
                    description: "Orphaned file: not referenced by any other documentation".to_string(),
                    suggestion: Some("Consider adding references to this file or removing it".to_string()),
                });
            }
        }

        if verbose {
            println!("📊 Validation Results:");
            println!("  Total files: {}", graph.metrics.total_files);
            println!("  Total references: {}", graph.metrics.total_references);
            println!("  Broken references: {}", graph.metrics.broken_references);
            println!("  Issues found: {}", issues.len());
        }

        Ok(issues)
    }
}

#[derive(Debug)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub file: PathBuf,
    pub line: usize,
    pub description: String,
    pub suggestion: Option<String>,
}

#[derive(Debug)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueSeverity::Error => write!(f, "ERROR"),
            IssueSeverity::Warning => write!(f, "WARNING"), 
            IssueSeverity::Info => write!(f, "INFO"),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { docs_path, output, include_source } => {
            let scanner = DocumentationScanner::new(include_source);
            let graph = scanner.scan_directory(&docs_path)?;
            
            let json = serde_json::to_string_pretty(&graph)?;
            fs::write(&output, json)?;
            
            println!("✅ Documentation graph saved to {}", output.display());
            println!("📊 Scanned {} files with {} references", 
                     graph.metrics.total_files, 
                     graph.metrics.total_references);
            
            if graph.metrics.broken_references > 0 {
                println!("⚠️  Found {} broken references", graph.metrics.broken_references);
            }
        }
        
        Commands::Watch { docs_path: _, dry_run: _, auto_threshold: _ } => {
            println!("🚧 Watch mode is not yet implemented");
            println!("This feature will be added in a future release");
            return Ok(());
        }
        
        Commands::Validate { docs_path, fix: _, verbose } => {
            let validator = DocumentationValidator::new();
            let issues = validator.validate(&docs_path, verbose)?;
            
            if issues.is_empty() {
                println!("✅ All documentation is consistent!");
            } else {
                for issue in &issues {
                    println!("{} {}:{} - {}", 
                             match issue.severity {
                                 IssueSeverity::Error => "❌",
                                 IssueSeverity::Warning => "⚠️",
                                 IssueSeverity::Info => "ℹ️",
                             },
                             issue.file.display(),
                             if issue.line > 0 { issue.line.to_string() } else { "".to_string() },
                             issue.description);
                    
                    if let Some(suggestion) = &issue.suggestion {
                        println!("   💡 {}", suggestion);
                    }
                }
                
                let error_count = issues.iter().filter(|i| matches!(i.severity, IssueSeverity::Error)).count();
                if error_count > 0 {
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Metrics { docs_path, format } => {
            let scanner = DocumentationScanner::new(false);
            let graph = scanner.scan_directory(&docs_path)?;
            
            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&graph.metrics)?);
                }
                "markdown" => {
                    println!("# Documentation Metrics\n");
                    println!("- **Total Files**: {}", graph.metrics.total_files);
                    println!("- **Total References**: {}", graph.metrics.total_references);
                    println!("- **Broken References**: {}", graph.metrics.broken_references);
                    
                    if !graph.metrics.most_referenced_files.is_empty() {
                        println!("\n## Most Referenced Files\n");
                        for (file, count) in &graph.metrics.most_referenced_files {
                            println!("- `{}`: {} references", file.display(), count);
                        }
                    }
                    
                    if !graph.metrics.reference_type_counts.is_empty() {
                        println!("\n## Reference Types\n");
                        for (ref_type, count) in &graph.metrics.reference_type_counts {
                            println!("- {:?}: {}", ref_type, count);
                        }
                    }
                }
                _ => return Err("Unsupported format. Use 'json' or 'markdown'".into()),
            }
        }
    }

    Ok(())
}