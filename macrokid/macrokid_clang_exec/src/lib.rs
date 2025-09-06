use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClangExecError {
    #[error("clang not found or failed to execute: {0}")] Exec(String),
    #[error("clang returned non-zero status: {0}")] Status(String),
    #[error("invalid JSON from clang: {0}")] Json(String),
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HeaderIR { pub structs: Vec<StructIR> }

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StructIR { pub name: String, pub attrs: Vec<AttrIR>, pub fields: Vec<FieldIR> }

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FieldIR { pub name: String, pub type_name: String, pub attrs: Vec<AttrIR> }

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AttrIR {
    pub kind: String,
    pub name: Option<String>,
    pub annotation: Option<String>,
    pub args: Vec<String>,
}

/// Analyze a C/C++ header by shelling out to clang and parsing its JSON AST dump.
/// This is a best-effort PoC; it extracts RecordDecl (struct/class) and FieldDecl with basic type names.
pub fn analyze_header<P: AsRef<Path>>(path: P, extra_args: &[&str]) -> Result<HeaderIR, ClangExecError> {
    let path = path.as_ref();
    let lossy = path.to_string_lossy();
    let mut args = vec![
        "-Xclang", "-ast-dump=json",
        "-fsyntax-only",
        lossy.as_ref(),
    ];
    args.extend(extra_args.iter().copied());

    let output = Command::new("clang")
        .args(&args)
        .output()
        .map_err(|e| ClangExecError::Exec(e.to_string()))?;
    if !output.status.success() {
        return Err(ClangExecError::Status(String::from_utf8_lossy(&output.stderr).into()));
    }
    let v: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ClangExecError::Json(e.to_string()))?;
    let mut ir = HeaderIR::default();
    collect_from_ast(&v, &mut ir);
    Ok(ir)
}

fn collect_from_ast(v: &Value, ir: &mut HeaderIR) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(kind)) = map.get("kind") {
                if kind.as_str() == "RecordDecl" {
                    // name if present
                    let name = map.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    if !name.is_empty() {
                        let attrs = collect_attrs_from_node(map);
                        let mut st = StructIR { name, attrs, fields: Vec::new() };
                        if let Some(inner) = map.get("inner").and_then(|x| x.as_array()) {
                            for node in inner {
                                if let Some(f) = parse_field_decl(node) { st.fields.push(f); }
                            }
                        }
                        ir.structs.push(st);
                    }
                }
            }
            for (_k, val) in map {
                collect_from_ast(val, ir);
            }
        }
        Value::Array(arr) => {
            for item in arr { collect_from_ast(item, ir); }
        }
        _ => {}
    }
}

fn parse_field_decl(node: &Value) -> Option<FieldIR> {
    if let Value::Object(m) = node {
        if m.get("kind").and_then(|k| k.as_str()) == Some("FieldDecl") {
            let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let type_name = m.get("type")
                .and_then(|t| t.get("qualType"))
                .and_then(|qt| qt.as_str())
                .unwrap_or("")
                .to_string();
            let attrs = collect_attrs_from_node(m);
            return Some(FieldIR { name, type_name, attrs });
        }
    }
    None
}

fn collect_attrs_from_node(map: &serde_json::Map<String, Value>) -> Vec<AttrIR> {
    let mut out = Vec::new();
    // Common location 1: "attributes": [ ... ]
    if let Some(Value::Array(attrs)) = map.get("attributes") {
        for a in attrs { if let Some(attr) = parse_attr(a) { out.push(attr); } }
    }
    // Common location 2: nested in "inner" as *Attr nodes
    if let Some(Value::Array(inner)) = map.get("inner") {
        for n in inner {
            if let Some(attr) = parse_attr(n) { out.push(attr); }
        }
    }
    out
}

fn parse_attr(v: &Value) -> Option<AttrIR> {
    let m = v.as_object()?;
    let kind = m.get("kind").and_then(|k| k.as_str())?.to_string();
    if !kind.ends_with("Attr") { return None; }
    let name = m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
    let annotation = m.get("annotation").and_then(|n| n.as_str()).map(|s| s.to_string());
    // Collect simple string/int args if present under a few likely keys
    let mut args = Vec::new();
    if let Some(Value::Array(arr)) = m.get("args") {
        for a in arr {
            if let Some(s) = a.as_str() { args.push(s.to_string()); }
            else if let Some(n) = a.as_i64() { args.push(n.to_string()); }
        }
    }
    Some(AttrIR { kind, name, annotation, args })
}

/// Minimal C++ header emitter for the extracted IR.
pub fn emit_cpp_header(ir: &HeaderIR, ns: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("// Generated by macrokid_clang_exec PoC\n");
    if let Some(ns) = ns { out.push_str(&format!("namespace {} {{\n", ns)); }
    for s in &ir.structs {
        // Emit attributes as comments for visibility in the PoC
        for a in &s.attrs {
            out.push_str(&format!("// @[{}] name={:?} annotation={:?} args={:?}\n", a.kind, a.name, a.annotation, a.args));
        }
        out.push_str(&format!("struct {} {{\n", s.name));
        for f in &s.fields {
            for a in &f.attrs {
                out.push_str(&format!("    // @[{}] name={:?} annotation={:?} args={:?}\n", a.kind, a.name, a.annotation, a.args));
            }
            out.push_str(&format!("    {} {};\n", f.type_name, f.name));
        }
        out.push_str("};\n\n");
    }
    if ns.is_some() { out.push_str("} // namespace\n"); }
    out
}

// ================= mk:: annotation parsing =================

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MkAnnotationKind { Struct, Vertex, Resource, Other(String) }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MkAnnotation {
    pub kind: MkAnnotationKind,
    pub args: BTreeMap<String, String>,
    pub raw: String,
}

fn is_ident_start(c: char) -> bool { c.is_ascii_alphabetic() || c == '_' }
fn is_ident_char(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' }

/// Parse strings like: `mk::vertex(location=1,format=vec3)` into a typed form.
pub fn parse_mk_annotation(s: &str) -> Option<MkAnnotation> {
    let s = s.trim();
    if !s.starts_with("mk::") { return None; }
    let mut i = 4usize; // after mk::
    let chars: Vec<char> = s.chars().collect();
    if i >= chars.len() || !is_ident_start(chars[i]) { return None; }
    let start = i;
    i += 1;
    while i < chars.len() && is_ident_char(chars[i]) { i += 1; }
    let name: String = chars[start..i].iter().collect();
    let kind = match name.as_str() {
        "struct" => MkAnnotationKind::Struct,
        "vertex" => MkAnnotationKind::Vertex,
        "resource" => MkAnnotationKind::Resource,
        other => MkAnnotationKind::Other(other.to_string()),
    };
    // Skip spaces
    while i < chars.len() && chars[i].is_whitespace() { i += 1; }
    let mut args = BTreeMap::new();
    if i < chars.len() && chars[i] == '(' {
        i += 1; // consume '('
        loop {
            // skip spaces
            while i < chars.len() && chars[i].is_whitespace() { i += 1; }
            if i >= chars.len() { return None; }
            if chars[i] == ')' { break; }
            // parse key
            if !is_ident_start(chars[i]) { return None; }
            let ks = i; i += 1; while i < chars.len() && is_ident_char(chars[i]) { i += 1; }
            let key: String = chars[ks..i].iter().collect();
            while i < chars.len() && chars[i].is_whitespace() { i += 1; }
            if i >= chars.len() || chars[i] != '=' { return None; }
            i += 1; // '='
            while i < chars.len() && chars[i].is_whitespace() { i += 1; }
            // parse value: quoted string or ident/number
            let val = if i < chars.len() && chars[i] == '"' {
                i += 1; let vs = i; while i < chars.len() && chars[i] != '"' { i += 1; }
                if i >= chars.len() { return None; }
                let v: String = chars[vs..i].iter().collect(); i += 1; v
            } else {
                let vs = i; while i < chars.len() && ![',', ')'].contains(&chars[i]) { i += 1; }
                let v: String = chars[vs..i].iter().collect::<String>().trim().to_string(); v
            };
            args.insert(key, val);
            while i < chars.len() && chars[i].is_whitespace() { i += 1; }
            if i < chars.len() && chars[i] == ',' { i += 1; continue; }
            if i < chars.len() && chars[i] == ')' { break; }
            if i >= chars.len() { break; }
        }
    }
    Some(MkAnnotation { kind, args, raw: s.to_string() })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedHeaderMk { pub structs: Vec<ParsedStructMk> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedStructMk { pub name: String, pub struct_attrs: Vec<MkAnnotation>, pub fields: Vec<ParsedFieldMk> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedFieldMk { pub name: String, pub type_name: String, pub field_attrs: Vec<MkAnnotation> }

/// Parse all mk:: annotations from a HeaderIR into typed forms.
pub fn parse_all_mk(ir: &HeaderIR) -> ParsedHeaderMk {
    let mut out = ParsedHeaderMk::default();
    for s in &ir.structs {
        let sa: Vec<MkAnnotation> = s
            .attrs
            .iter()
            .filter_map(|a| a.annotation.as_deref())
            .filter_map(parse_mk_annotation)
            .collect();
        let mut fields = Vec::new();
        for f in &s.fields {
            let fa: Vec<MkAnnotation> = f
                .attrs
                .iter()
                .filter_map(|a| a.annotation.as_deref())
                .filter_map(parse_mk_annotation)
                .collect();
            fields.push(ParsedFieldMk { name: f.name.clone(), type_name: f.type_name.clone(), field_attrs: fa });
        }
        out.structs.push(ParsedStructMk { name: s.name.clone(), struct_attrs: sa, fields });
    }
    out
}

// ================= C-only IR (functions/enums/typedefs/structs) =================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CHeaderIR {
    pub structs: Vec<CStructIR>,
    pub enums: Vec<CEnumIR>,
    pub typedefs: Vec<CTypedefIR>,
    pub functions: Vec<CFunctionIR>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CStructIR { pub name: String, pub is_union: bool, pub fields: Vec<CFieldIR> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CFieldIR { pub name: String, pub type_name: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CEnumIR { pub name: String, pub items: Vec<(String, String)> }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CTypedefIR { pub name: String, pub underlying: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CFunctionIR { pub name: String, pub ret: String, pub params: Vec<(String, String)> }

/// Analyze a header as C and extract C-only IR (structs/enums/typedefs/functions).
pub fn analyze_header_c<P: AsRef<Path>>(path: P, extra_args: &[&str]) -> Result<CHeaderIR, ClangExecError> {
    let path = path.as_ref();
    let lossy = path.to_string_lossy();
    let mut args = vec![
        "-x", "c",
        "-Xclang", "-ast-dump=json",
        "-fsyntax-only",
        lossy.as_ref(),
    ];
    args.extend(extra_args.iter().copied());
    let output = Command::new("clang")
        .args(&args)
        .output()
        .map_err(|e| ClangExecError::Exec(e.to_string()))?;
    if !output.status.success() {
        return Err(ClangExecError::Status(String::from_utf8_lossy(&output.stderr).into()));
    }
    let v: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ClangExecError::Json(e.to_string()))?;
    let mut ir = CHeaderIR::default();
    collect_c_from_ast(&v, &mut ir);
    Ok(ir)
}

fn collect_c_from_ast(v: &Value, ir: &mut CHeaderIR) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(kind)) = map.get("kind") {
                match kind.as_str() {
                    "RecordDecl" => {
                        let name = map.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if !name.is_empty() {
                            let tag = map.get("tagUsed").and_then(|t| t.as_str()).unwrap_or("struct");
                            let is_union = tag == "union";
                            let mut fields = Vec::new();
                            if let Some(inner) = map.get("inner").and_then(|x| x.as_array()) {
                                for node in inner { if let Some(f) = parse_c_field(node) { fields.push(f); } }
                            }
                            ir.structs.push(CStructIR { name: name.to_string(), is_union, fields });
                        }
                    }
                    "EnumDecl" => {
                        let name = map.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if !name.is_empty() {
                            let mut items = Vec::new();
                            if let Some(inner) = map.get("inner").and_then(|x| x.as_array()) {
                                for node in inner {
                                    if let Some(it) = parse_c_enum_const(node) { items.push(it); }
                                }
                            }
                            ir.enums.push(CEnumIR { name: name.to_string(), items });
                        }
                    }
                    "TypedefDecl" => {
                        let name = map.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if !name.is_empty() {
                            let underlying = map.get("type")
                                .and_then(|t| t.get("qualType").or_else(|| t.get("desugaredQualType")))
                                .and_then(|qt| qt.as_str())
                                .or_else(|| map.get("underlyingType").and_then(|t| t.get("qualType")).and_then(|qt| qt.as_str()))
                                .unwrap_or("");
                            ir.typedefs.push(CTypedefIR { name: name.to_string(), underlying: underlying.to_string() });
                        }
                    }
                    "FunctionDecl" => {
                        let name = map.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        if !name.is_empty() {
                            let ty = map.get("type").and_then(|t| t.get("qualType")).and_then(|qt| qt.as_str()).unwrap_or("");
                            let ret = ty.split_once('(').map(|(r, _)| r.trim()).unwrap_or(ty).to_string();
                            let mut params = Vec::new();
                            if let Some(inner) = map.get("inner").and_then(|x| x.as_array()) {
                                for node in inner {
                                    if let Some(p) = parse_c_param(node) { params.push(p); }
                                }
                            }
                            ir.functions.push(CFunctionIR { name: name.to_string(), ret, params });
                        }
                    }
                    _ => {}
                }
            }
            for (_k, val) in map { collect_c_from_ast(val, ir); }
        }
        Value::Array(arr) => { for item in arr { collect_c_from_ast(item, ir); } }
        _ => {}
    }
}

fn parse_c_field(node: &Value) -> Option<CFieldIR> {
    if let Value::Object(m) = node {
        if m.get("kind").and_then(|k| k.as_str()) == Some("FieldDecl") {
            let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let type_name = m.get("type").and_then(|t| t.get("qualType")).and_then(|qt| qt.as_str()).unwrap_or("").to_string();
            return Some(CFieldIR { name, type_name });
        }
    }
    None
}

fn parse_c_enum_const(node: &Value) -> Option<(String, String)> {
    if let Value::Object(m) = node {
        if m.get("kind").and_then(|k| k.as_str()) == Some("EnumConstantDecl") {
            let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            // Try several places for value
            let val = m.get("value").and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
                .or_else(|| m.get("inner").and_then(|inn| inn.as_array()).and_then(|arr| arr.iter().find_map(|e| e.get("value")).and_then(|v| v.as_i64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))))
                .unwrap_or_default();
            return Some((name, val));
        }
    }
    None
}

fn parse_c_param(node: &Value) -> Option<(String, String)> {
    if let Value::Object(m) = node {
        if m.get("kind").and_then(|k| k.as_str()) == Some("ParmVarDecl") {
            let name = m.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let type_name = m.get("type").and_then(|t| t.get("qualType")).and_then(|qt| qt.as_str()).unwrap_or("").to_string();
            return Some((name, type_name));
        }
    }
    None
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MacroIR { pub name: String, pub value: String }

/// Extract preprocessor macros by invoking `clang -dM -E`.
pub fn analyze_macros_c<P: AsRef<Path>>(header: P, extra_args: &[&str]) -> Result<Vec<MacroIR>, ClangExecError> {
    let header = header.as_ref();
    let lossy = header.to_string_lossy();
    let mut args = vec![
        "-dM", "-E",
        "-x", "c",
        "-include", lossy.as_ref(),
        "/dev/null",
    ];
    args.extend(extra_args.iter().copied());
    let output = Command::new("clang")
        .args(&args)
        .output()
        .map_err(|e| ClangExecError::Exec(e.to_string()))?;
    if !output.status.success() {
        return Err(ClangExecError::Status(String::from_utf8_lossy(&output.stderr).into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("#define ") {
            if let Some((name, value)) = rest.split_once(' ') {
                out.push(MacroIR { name: name.to_string(), value: value.to_string() });
            } else {
                out.push(MacroIR { name: rest.to_string(), value: String::new() });
            }
        }
    }
    Ok(out)
}
