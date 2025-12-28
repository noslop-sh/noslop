//! Integration tests for parser module

use noslop::parser::{Match, Pattern, Span, Token, TokenKind, TokenTree};
use std::path::Path;

// Span tests
#[test]
fn test_span_new() {
    let span = Span::new(10, 20);
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
}

#[test]
fn test_span_len() {
    let span = Span::new(10, 25);
    assert_eq!(span.len(), 15);
}

#[test]
fn test_span_is_empty() {
    assert!(Span::new(10, 10).is_empty());
    assert!(!Span::new(10, 11).is_empty());
}

#[test]
fn test_span_contains() {
    let outer = Span::new(0, 100);
    let inner = Span::new(10, 20);
    let outside = Span::new(50, 150);

    assert!(outer.contains(&inner));
    assert!(!outer.contains(&outside));
    assert!(!inner.contains(&outer));
}

// Token tests
#[test]
fn test_token_kind_display() {
    assert_eq!(TokenKind::Function.to_string(), "function");
    assert_eq!(TokenKind::Class.to_string(), "class");
    assert_eq!(TokenKind::Variable.to_string(), "variable");
}

#[test]
fn test_token_new() {
    let span = Span::new(0, 10);
    let token = Token::new(TokenKind::Function, "test_fn".to_string(), span);

    assert_eq!(token.kind, TokenKind::Function);
    assert_eq!(token.name, "test_fn");
    assert_eq!(token.span.start, 0);
    assert!(token.children.is_empty());
    assert!(token.doc.is_none());
}

#[test]
fn test_token_with_child() {
    let parent = Token::new(TokenKind::Class, "MyClass".to_string(), Span::new(0, 100));
    let child = Token::new(TokenKind::Function, "method".to_string(), Span::new(10, 50));

    let parent = parent.with_child(child);
    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].name, "method");
}

#[test]
fn test_token_with_doc() {
    let token = Token::new(TokenKind::Function, "test".to_string(), Span::new(0, 10))
        .with_doc("Test documentation".to_string());

    assert_eq!(token.doc, Some("Test documentation".to_string()));
}

#[test]
fn test_token_find_child() {
    let child1 = Token::new(TokenKind::Function, "method1".to_string(), Span::new(0, 10));
    let child2 = Token::new(TokenKind::Function, "method2".to_string(), Span::new(10, 20));

    let parent = Token::new(TokenKind::Class, "MyClass".to_string(), Span::new(0, 100))
        .with_child(child1)
        .with_child(child2);

    assert!(parent.find_child("method1").is_some());
    assert!(parent.find_child("method2").is_some());
    assert!(parent.find_child("nonexistent").is_none());
}

#[test]
fn test_token_full_name() {
    let token = Token::new(TokenKind::Function, "method".to_string(), Span::new(0, 10));

    assert_eq!(token.full_name(None), "method");
    assert_eq!(token.full_name(Some("MyClass")), "MyClass::method");
}

// TokenTree tests
#[test]
fn test_token_tree_empty() {
    let tree = TokenTree::empty();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
}

#[test]
fn test_token_tree_push() {
    let mut tree = TokenTree::empty();
    let token = Token::new(TokenKind::Function, "test".to_string(), Span::new(0, 10));

    tree.push(token);
    assert_eq!(tree.len(), 1);
    assert!(!tree.is_empty());
}

#[test]
fn test_token_tree_find_by_name() {
    let token1 = Token::new(TokenKind::Function, "foo".to_string(), Span::new(0, 10));
    let token2 = Token::new(TokenKind::Function, "bar".to_string(), Span::new(10, 20));

    let tree = TokenTree::new(vec![token1, token2]);

    assert!(tree.find_by_name("foo").is_some());
    assert!(tree.find_by_name("bar").is_some());
    assert!(tree.find_by_name("baz").is_none());
}

#[test]
fn test_token_tree_find_by_name_nested() {
    let child = Token::new(TokenKind::Function, "nested".to_string(), Span::new(5, 15));
    let parent =
        Token::new(TokenKind::Class, "Parent".to_string(), Span::new(0, 20)).with_child(child);

    let tree = TokenTree::new(vec![parent]);

    assert!(tree.find_by_name("Parent").is_some());
    assert!(tree.find_by_name("nested").is_some());
}

#[test]
fn test_token_tree_find_by_kind() {
    let func1 = Token::new(TokenKind::Function, "func1".to_string(), Span::new(0, 10));
    let func2 = Token::new(TokenKind::Function, "func2".to_string(), Span::new(10, 20));
    let class1 = Token::new(TokenKind::Class, "MyClass".to_string(), Span::new(20, 30));

    let tree = TokenTree::new(vec![func1, func2, class1]);

    let functions = tree.find_by_kind(TokenKind::Function);
    assert_eq!(functions.len(), 2);

    let classes = tree.find_by_kind(TokenKind::Class);
    assert_eq!(classes.len(), 1);
}

#[test]
fn test_token_tree_len_with_children() {
    let child1 = Token::new(TokenKind::Function, "method1".to_string(), Span::new(0, 10));
    let child2 = Token::new(TokenKind::Function, "method2".to_string(), Span::new(10, 20));
    let parent = Token::new(TokenKind::Class, "MyClass".to_string(), Span::new(0, 100))
        .with_child(child1)
        .with_child(child2);

    let tree = TokenTree::new(vec![parent]);
    assert_eq!(tree.len(), 3); // 1 parent + 2 children
}

// Pattern tests
#[test]
fn test_pattern_kind_display() {
    use noslop::parser::PatternKind;
    assert_eq!(PatternKind::Regex.to_string(), "regex");
    assert_eq!(PatternKind::Glob.to_string(), "glob");
    assert_eq!(PatternKind::Ast.to_string(), "ast");
    assert_eq!(PatternKind::Semantic.to_string(), "semantic");
}

#[test]
fn test_pattern_regex_constructor() {
    let pattern = Pattern::regex("test-id", r"fn\s+\w+", "Test pattern");
    assert_eq!(pattern.id, "test-id");
    assert_eq!(pattern.pattern, r"fn\s+\w+");
    assert_eq!(pattern.message, "Test pattern");
    assert_eq!(pattern.severity, "warn");
}

#[test]
fn test_pattern_with_severity() {
    let pattern = Pattern::regex("test", "pattern", "message").with_severity("block");
    assert_eq!(pattern.severity, "block");
}

#[test]
fn test_pattern_with_languages() {
    let pattern = Pattern::regex("test", "pattern", "message")
        .with_languages(vec!["rust".to_string(), "python".to_string()]);
    assert_eq!(pattern.languages.len(), 2);
    assert!(pattern.languages.contains(&"rust".to_string()));
}

#[test]
fn test_pattern_applies_to_rust_file() {
    let pattern =
        Pattern::regex("test", "pattern", "message").with_languages(vec!["rust".to_string()]);

    assert!(pattern.applies_to("src/main.rs"));
    assert!(!pattern.applies_to("src/main.py"));
}

#[test]
fn test_pattern_applies_to_multiple_languages() {
    let pattern = Pattern::regex("test", "pattern", "message")
        .with_languages(vec!["rust".to_string(), "python".to_string()]);

    assert!(pattern.applies_to("src/main.rs"));
    assert!(pattern.applies_to("src/main.py"));
    assert!(!pattern.applies_to("src/main.js"));
}

#[test]
fn test_pattern_applies_to_files_glob() {
    let mut pattern = Pattern::regex("test", "pattern", "message");
    pattern.files = vec!["*test".to_string()];

    assert!(pattern.applies_to("src/my_test"));
    assert!(pattern.applies_to("unit_test"));
    assert!(!pattern.applies_to("src/main.rs"));

    // Test prefix glob
    let mut pattern2 = Pattern::regex("test", "pattern", "message");
    pattern2.files = vec!["test*".to_string()];
    assert!(pattern2.applies_to("test_utils.rs"));
    assert!(pattern2.applies_to("testing.rs"));
    assert!(!pattern2.applies_to("src/test.rs"));

    // Test contains (no wildcards)
    let mut pattern3 = Pattern::regex("test", "pattern", "message");
    pattern3.files = vec!["test".to_string()];
    assert!(pattern3.applies_to("src/test_utils.rs"));
    assert!(pattern3.applies_to("testing.rs"));
    assert!(!pattern3.applies_to("src/main.rs"));
}

#[test]
fn test_pattern_applies_to_no_filters() {
    let pattern = Pattern::regex("test", "pattern", "message");
    assert!(pattern.applies_to("any/file.txt"));
}

// Match tests
#[test]
fn test_match_new() {
    let span = Span { start: 0, end: 10 };
    let m = Match::new(span, "test text".to_string(), "pattern-id".to_string());

    assert_eq!(m.span.start, 0);
    assert_eq!(m.span.end, 10);
    assert_eq!(m.text, "test text");
    assert_eq!(m.pattern_id, "pattern-id");
}

#[test]
fn test_match_line_number() {
    let content = "line 1\nline 2\nline 3";
    let span = Span { start: 7, end: 13 }; // "line 2"
    let m = Match::new(span, "line 2".to_string(), "test".to_string());

    assert_eq!(m.line_number(content), 2);
}

#[test]
fn test_match_line_number_first_line() {
    let content = "line 1\nline 2";
    let span = Span { start: 0, end: 6 };
    let m = Match::new(span, "line 1".to_string(), "test".to_string());

    assert_eq!(m.line_number(content), 1);
}

// Parser registry and regex parser tests
#[test]
fn test_regex_parser_match_pattern() {
    use noslop::parser::ParserRegistry;

    let registry = ParserRegistry::new();
    let parser = registry.regex_parser();

    let pattern = Pattern::regex("test", r"fn\s+\w+", "Find functions");
    let content = b"fn main() { fn helper() {} }";

    let matches = parser.match_pattern(content, &pattern).unwrap();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].text, "fn main");
    assert_eq!(matches[1].text, "fn helper");
}

#[test]
fn test_parser_registry_get_regex_parser() {
    use noslop::parser::ParserRegistry;

    let registry = ParserRegistry::new();
    let parser = registry.regex_parser();
    assert_eq!(parser.name(), "regex");
}

#[test]
fn test_parser_registry_parser_for_unknown_extension() {
    use noslop::parser::ParserRegistry;

    let registry = ParserRegistry::new();
    let path = Path::new("test.xyz");
    assert!(registry.parser_for(path).is_none());
}

#[test]
fn test_regex_parser_tokenize_returns_empty() {
    use noslop::parser::ParserRegistry;

    let registry = ParserRegistry::new();
    let parser = registry.regex_parser();

    let tree = parser.tokenize(b"some content", Path::new("test.txt")).unwrap();
    assert!(tree.is_empty());
}

#[test]
fn test_regex_parser_find_token_returns_none() {
    use noslop::parser::ParserRegistry;

    let registry = ParserRegistry::new();
    let parser = registry.regex_parser();

    let result = parser.find_token(b"some content", "any_name").unwrap();
    assert!(result.is_none());
}
