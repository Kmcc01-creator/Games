use perl_regex_poc::{regex_match, regex_subst};
use perl_regex_runtime::{PerlRegexMatch, PerlRegexSubst};

fn main() {
    let text = "Hello, World! The year is 2024.";
    
    println!("=== Perl-like Regex Demo ===\n");
    
    // Pattern matching (simplified syntax for proof of concept)
    let word_match = regex_match!(text, r"\b[A-Z][a-z]+\b");
    println!("Word pattern match: {}", word_match.matched);
    if let Some(m) = word_match.full_match() {
        println!("First match: '{}'", m);
    }
    
    let year_match = regex_match!(text, r"\d{4}");
    println!("Year pattern match: {}", year_match.matched);
    if let Some(year) = year_match.full_match() {
        println!("Found year: {}", year);
    }
    
    println!();
    
    // Substitution
    let subst_result = regex_subst!(text, r"\d{4}", "2025");
    println!("Original: {}", text);
    println!("After substitution: {}", subst_result.result);
    println!("Substitutions made: {}", subst_result.count);
    
    println!();
    
    // More complex example
    let email_text = "Contact: john@example.com or support@company.org";
    let email_match = regex_match!(email_text, r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}");
    println!("Email found: {}", email_match.matched);
    
    let censored = regex_subst!(email_text, r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", "[EMAIL]");
    println!("Censored: {}", censored.result);
}

// This demonstrates what the eventual full Perl syntax could look like:
// 
// fn future_perl_syntax() {
//     let text = "Hello, World! The year is 2024.";
//     
//     // Full Perl-like syntax (future goal)
//     let matches = perl_regex! { text =~ /(\w+),\s*(\w+)!/g };  
//     println!("First word: {}, Second word: {}", matches.$1, matches.$2);
//     
//     let result = perl_regex! { text =~ s/\d{4}/2025/g };
//     println!("Result: {}, substitutions: {}", result, result.count);
//     
//     // Even more advanced
//     let captures = perl_regex! {
//         text =~ m{
//             (?P<greeting>\w+),\s*
//             (?P<target>\w+)!.*?
//             (?P<year>\d{4})
//         }x  // Extended regex syntax
//     };
//     println!("Greeting: {}, Target: {}, Year: {}", 
//              captures.greeting, captures.target, captures.year);
// }
