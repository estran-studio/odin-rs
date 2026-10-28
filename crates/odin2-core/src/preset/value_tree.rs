//! Décodage structurel du flux JUCE ValueTree (pas de recherche de marqueurs).
//! Format: type UTF-8 terminé par zéro, compte compressé, propriétés var,
//! compte compressé des enfants, puis enfants récursifs. Voir JUCE
//! juce_ValueTree.cpp et juce_Variant.cpp, writeToStream/readFromStream.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::io::{self, Read, Seek};

#[derive(Debug)]
pub enum ValueTreeError {
    Io(io::Error),
    InvalidFormat(String),
    UnexpectedEof,
    InvalidUtf8,
    UnsupportedType(u8),
}
impl From<io::Error> for ValueTreeError {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}
impl std::fmt::Display for ValueTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::InvalidFormat(e) => write!(f, "Invalid format: {e}"),
            Self::UnexpectedEof => f.write_str("Unexpected end of file"),
            Self::InvalidUtf8 => f.write_str("Invalid UTF-8 string"),
            Self::UnsupportedType(t) => write!(f, "Unsupported value type: {t}"),
        }
    }
}
impl std::error::Error for ValueTreeError {}

#[derive(Debug, Clone, PartialEq)]
pub enum VariantValue {
    Void,
    Undefined,
    Int(i32),
    Int64(i64),
    Double(f64),
    String(String),
    Bool(bool),
    Array(Vec<VariantValue>),
    Binary(Vec<u8>),
}
impl VariantValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Int64(v) => Some(*v as f64),
            Self::Double(v) => Some(*v),
            Self::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            Self::String(v) => v.trim().parse().ok(),
            _ => None,
        }
    }
    pub fn as_f32(&self) -> Option<f32> { self.as_f64().map(|v| v as f32) }
    pub fn as_i32(&self) -> Option<i32> { self.as_f64().map(|v| v as i32) }
    pub fn as_string(&self) -> Option<&str> {
        if let Self::String(v) = self { Some(v) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::String(v) = self {
            return Some(v.trim().eq_ignore_ascii_case("true") || v.trim().eq_ignore_ascii_case("yes") || v.trim().parse::<i64>().unwrap_or(0) != 0);
        }
        self.as_f64().map(|v| v != 0.0)
    }
}

/// Arbre complet: les nœuds inconnus ne sont pas perdus et restent auditables.
#[derive(Debug, Clone, Default)]
pub struct ValueTreeNode {
    pub node_type: String,
    pub properties: BTreeMap<String, VariantValue>,
    pub children: Vec<ValueTreeNode>,
}

#[derive(Debug, Clone, Default)]
pub struct ValueTree {
    pub root: ValueTreeNode,
    pub params: BTreeMap<String, f64>,
    pub fx: BTreeMap<String, VariantValue>,
    pub lfo: BTreeMap<String, VariantValue>,
    pub misc: BTreeMap<String, VariantValue>,
    pub mod_matrix: BTreeMap<String, VariantValue>,
    pub osc: BTreeMap<String, VariantValue>,
    pub draw: BTreeMap<String, VariantValue>,
    /// Propriétés/nœuds hors schéma, avec leur chemin réel dans l'arbre.
    pub unmapped: Vec<String>,
}
impl ValueTree {
    pub fn from_bytes(data: &[u8]) -> Result<Self, ValueTreeError> {
        let mut decoder = Decoder { data, pos: 0 };
        let root = decoder.node(0)?;
        if decoder.pos != data.len() {
            return Err(ValueTreeError::InvalidFormat(format!("{} trailing bytes", data.len() - decoder.pos)));
        }
        if root.node_type != "Odin" {
            return Err(ValueTreeError::InvalidFormat("root is not Odin".to_string()));
        }
        let mut tree = Self { root, ..Self::default() };
        for name in tree.root.properties.keys() { tree.unmapped.push(format!("Odin/{name}")); }
        for (index, child) in tree.root.children.iter().enumerate() {
            let path = format!("Odin/{index}:{}", child.node_type);
            if child.node_type == "PARAM" {
                let id = child.properties.get("id").and_then(VariantValue::as_string);
                let value = child.properties.get("value").and_then(VariantValue::as_f64);
                if let (Some(id), Some(value)) = (id, value) {
                    if tree.params.insert(id.to_string(), value).is_some() {
                        return Err(ValueTreeError::InvalidFormat(format!("duplicate parameter {id}")));
                    }
                    for key in child.properties.keys().filter(|k| k.as_str() != "id" && k.as_str() != "value") {
                        tree.unmapped.push(format!("{path}/{key}"));
                    }
                } else { collect_paths(child, &path, &mut tree.unmapped); }
            } else {
                let target = match child.node_type.as_str() {
                    "fx" => Some(&mut tree.fx), "lfo" => Some(&mut tree.lfo),
                    "misc" => Some(&mut tree.misc), "mod" => Some(&mut tree.mod_matrix),
                    "osc" => Some(&mut tree.osc), "draw" => Some(&mut tree.draw),
                    _ => None,
                };
                if let Some(target) = target {
                    for (key, value) in &child.properties {
                        if target.insert(key.clone(), value.clone()).is_some() {
                            return Err(ValueTreeError::InvalidFormat(format!("duplicate property {path}/{key}")));
                        }
                    }
                } else { collect_paths(child, &path, &mut tree.unmapped); }
            }
            // No nested property is silently absorbed by a recognized parent.
            for (n, nested) in child.children.iter().enumerate() {
                collect_paths(nested, &format!("{path}/{n}:{}", nested.node_type), &mut tree.unmapped);
            }
        }
        Ok(tree)
    }
    pub fn read_from<R: Read + Seek>(reader: &mut R) -> Result<Self, ValueTreeError> {
        let mut bytes = Vec::new(); reader.read_to_end(&mut bytes)?; Self::from_bytes(&bytes)
    }
    pub fn get_param(&self, name: &str) -> Option<f64> { self.params.get(name).copied() }
    pub fn get_misc(&self, name: &str) -> Option<&VariantValue> { self.misc.get(name) }
    pub fn get_fx(&self, name: &str) -> Option<&VariantValue> { self.fx.get(name) }
    pub fn get_lfo(&self, name: &str) -> Option<&VariantValue> { self.lfo.get(name) }
    pub fn get_osc(&self, name: &str) -> Option<&VariantValue> { self.osc.get(name) }
    pub fn get_mod(&self, name: &str) -> Option<&VariantValue> { self.mod_matrix.get(name) }
}
fn collect_paths(node: &ValueTreeNode, path: &str, out: &mut Vec<String>) {
    if node.properties.is_empty() && node.children.is_empty() { out.push(path.to_string()); }
    for name in node.properties.keys() { out.push(format!("{path}/{name}")); }
    for (i, child) in node.children.iter().enumerate() {
        collect_paths(child, &format!("{path}/{i}:{}", child.node_type), out);
    }
}
struct Decoder<'a> { data: &'a [u8], pos: usize }
impl<'a> Decoder<'a> {
    fn bytes(&mut self, count: usize) -> Result<&'a [u8], ValueTreeError> {
        let end = self.pos.checked_add(count).ok_or(ValueTreeError::UnexpectedEof)?;
        let bytes = self.data.get(self.pos..end).ok_or(ValueTreeError::UnexpectedEof)?;
        self.pos = end; Ok(bytes)
    }
    fn byte(&mut self) -> Result<u8, ValueTreeError> { Ok(self.bytes(1)?[0]) }
    fn compressed(&mut self) -> Result<i32, ValueTreeError> {
        let header = self.byte()?;
        let count = (header & 0x7f) as usize;
        if count > 4 { return Err(ValueTreeError::InvalidFormat("invalid compressed integer".to_string())); }
        let mut value = 0u32;
        for i in 0..count { value |= (self.byte()? as u32) << (i * 8); }
        if value > i32::MAX as u32 { return Err(ValueTreeError::InvalidFormat("compressed integer overflow".to_string())); }
        Ok(if header & 0x80 != 0 { -(value as i32) } else { value as i32 })
    }
    fn count(&mut self) -> Result<usize, ValueTreeError> {
        let count = self.compressed()?;
        if count < 0 || count as usize > self.data.len().saturating_sub(self.pos) {
            return Err(ValueTreeError::InvalidFormat("invalid count".to_string()));
        }
        Ok(count as usize)
    }
    fn string(&mut self) -> Result<String, ValueTreeError> {
        let count = self.data[self.pos..].iter().position(|b| *b == 0).ok_or(ValueTreeError::UnexpectedEof)?;
        let bytes = self.bytes(count + 1)?;
        String::from_utf8(bytes[..count].to_vec()).map_err(|_| ValueTreeError::InvalidUtf8)
    }
    fn variant(&mut self, depth: usize) -> Result<VariantValue, ValueTreeError> {
        if depth > 128 { return Err(ValueTreeError::InvalidFormat("variant nesting limit".to_string())); }
        let count = self.count()?;
        if count == 0 { return Ok(VariantValue::Void); }
        let payload = self.bytes(count)?;
        let mut value = Decoder { data: payload, pos: 0 };
        let marker = value.byte()?;
        let result = match marker {
            1 => VariantValue::Int(i32::from_le_bytes(value.bytes(4)?.try_into().unwrap())),
            2 => VariantValue::Bool(true),
            3 => VariantValue::Bool(false),
            4 => VariantValue::Double(f64::from_le_bytes(value.bytes(8)?.try_into().unwrap())),
            5 => VariantValue::String(value.string()?),
            6 => VariantValue::Int64(i64::from_le_bytes(value.bytes(8)?.try_into().unwrap())),
            7 => {
                let count = value.count()?;
                let mut array = Vec::with_capacity(count);
                for _ in 0..count { array.push(value.variant(depth + 1)?); }
                VariantValue::Array(array)
            }
            8 => VariantValue::Binary(value.bytes(payload.len() - 1)?.to_vec()),
            9 => VariantValue::Undefined,
            _ => return Err(ValueTreeError::UnsupportedType(marker)),
        };
        if value.pos != payload.len() { return Err(ValueTreeError::InvalidFormat("variant length mismatch".to_string())); }
        Ok(result)
    }
    fn node(&mut self, depth: usize) -> Result<ValueTreeNode, ValueTreeError> {
        if depth > 128 { return Err(ValueTreeError::InvalidFormat("tree nesting limit".to_string())); }
        let node_type = self.string()?;
        if node_type.is_empty() { return Err(ValueTreeError::InvalidFormat("empty node type".to_string())); }
        let mut node = ValueTreeNode { node_type, ..ValueTreeNode::default() };
        let properties = self.count()?;
        for _ in 0..properties {
            let name = self.string()?;
            let value = self.variant(0)?;
            if node.properties.insert(name, value).is_some() { return Err(ValueTreeError::InvalidFormat("duplicate property".to_string())); }
        }
        let children = self.count()?;
        for _ in 0..children { node.children.push(self.node(depth + 1)?); }
        Ok(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn juce_true_false_and_int64_markers() {
        let bytes = b"Odin\0\x01\x03yes\0\x01\x01\x02no\0\x01\x01\x03wide\0\x01\x09\x06\x01\0\0\0\0\0\0\0\0";
        let tree = ValueTree::from_bytes(bytes).unwrap();
        assert_eq!(tree.root.properties["yes"], VariantValue::Bool(true));
        assert_eq!(tree.root.properties["no"], VariantValue::Bool(false));
        assert_eq!(tree.root.properties["wide"], VariantValue::Int64(1));
    }
    #[test]
    fn truncation_is_not_a_partial_success() {
        assert!(ValueTree::from_bytes(b"Odin\0\x01\x01x\0\x01\x09\x04").is_err());
    }

    #[test]
    fn compressed_property_count_larger_than_one_byte() {
        let mut bytes = b"Odin\0\0\x01\x01draw\0\x02\x2c\x01".to_vec();
        for i in 0..300i32 {
            bytes.extend_from_slice(format!("sample{i}\0").as_bytes());
            bytes.extend_from_slice(&[1, 5, 1]);
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        bytes.push(0);
        let tree = ValueTree::from_bytes(&bytes).unwrap();
        assert_eq!(tree.draw.len(), 300);
        assert_eq!(tree.draw["sample299"], VariantValue::Int(299));
    }

    #[test]
    fn unknown_descendant_is_reported_even_below_known_section() {
        let bytes = b"Odin\0\0\x01\x01fx\0\0\x01\x01future\0\x01\x01x\0\x01\x05\x01\x2a\0\0\0\0";
        let tree = ValueTree::from_bytes(bytes).unwrap();
        let (_, audit) = crate::preset::OdinPreset::from_value_tree_audited(&tree).unwrap();
        assert_eq!(audit.unread, ["Odin/0:fx/0:future/x"]);
    }
}
