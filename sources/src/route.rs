//! Module to manage the routes used by the different sources
//!

use std::collections::btree_map::{IntoValues, Iter, Keys, Values, ValuesMut};
use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

/// A struct that manages a collection of routes represented as key-value pairs.
///
/// Each route is stored as a pair of strings, where the key is the route name, 
/// and the value is the associated route string.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use fetiche_sources::Routes;
///
/// let mut routes = Routes::from(BTreeMap::from([
///     ("home".to_string(), "/home".to_string()),
///     ("about".to_string(), "/about".to_string()),
/// ]));
///
/// assert_eq!(routes.get("home"), Some(&"/home".to_string()));
/// assert!(routes.contains_key("about"));
/// ```
///
/// The `Routes` struct provides several utility methods for accessing, 
/// modifying, and iterating over the routes.
///
#[derive(Clone, Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct Routes(BTreeMap<String, String>);

impl Routes {
    /// Wrap `get`
    ///
    #[inline]
    pub fn get(&self, name: &str) -> Option<&String> {
        self.0.get(name)
    }

    /// Wrap `get_mut`
    ///
    #[inline]
    pub fn get_mut(&mut self, name: &str) -> Option<&mut String> {
        self.0.get_mut(name)
    }

    /// Wrap `is_empty()`
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Wrap `len()`
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Wrap `keys()`
    ///
    #[inline]
    pub fn keys(&self) -> Keys<'_, String, String> {
        self.0.keys()
    }

    /// Wrap `index()`
    ///
    #[inline]
    pub fn index(&self, s: &str) -> Option<&String> {
        self.0.get(s)
    }

    /// Wrap `index_mut()`
    ///
    #[inline]
    pub fn index_mut(&mut self, s: &str) -> Option<&String> {
        self.0.get(s)
    }

    /// Wrap `values()`
    ///
    #[inline]
    pub fn values(&self) -> Values<'_, String, String> {
        self.0.values()
    }

    /// Wrap `values_mut()`
    ///
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, String> {
        self.0.values_mut()
    }

    /// Wrap `into_values()`
    ///
    #[inline]
    pub fn into_values(self) -> IntoValues<String, String> {
        self.0.into_values()
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn contains_key(&self, s: &str) -> bool {
        self.0.contains_key(s)
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn iter(&self) -> Iter<'_, String, String> {
        self.0.iter()
    }
}

impl Index<&str> for Routes {
    type Output = String;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: &str) -> &Self::Output {
        self.0.get(s).unwrap()
    }
}

impl Index<String> for Routes {
    type Output = String;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: String) -> &Self::Output {
        self.0.get(&s).unwrap()
    }
}

impl IndexMut<&str> for Routes {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: &str) -> &mut Self::Output {
        let me = self.0.get_mut(s);
        if me.is_none() {
            self.0.insert(s.to_string(), String::new());
        }
        self.0.get_mut(s).unwrap()
    }
}

impl IndexMut<String> for Routes {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: String) -> &mut Self::Output {
        let me = self.0.get_mut(&s);
        if me.is_none() {
            self.0.insert(s.to_string(), String::new());
        }
        self.0.get_mut(&s).unwrap()
    }
}

impl<'a> IntoIterator for &'a Routes {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    /// We can now do `sources.iter()`
    ///
    fn into_iter(self) -> Iter<'a, String, String> {
        self.0.iter()
    }
}

impl From<BTreeMap<String, String>> for Routes {
    fn from(value: BTreeMap<String, String>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_get() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));

        assert_eq!(routes.get("home"), Some(&"/home".to_string()));
        assert_eq!(routes.get("about"), Some(&"/about".to_string()));
        assert_eq!(routes.get("contact"), None);
    }

    #[test]
    fn test_is_empty_and_len() {
        let empty_routes: Routes = Routes::from(BTreeMap::new());
        assert!(empty_routes.is_empty());
        assert_eq!(empty_routes.len(), 0);

        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
        ]));
        assert!(!routes.is_empty());
        assert_eq!(routes.len(), 1);
    }

    #[test]
    fn test_keys() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));
        let keys: Vec<&String> = routes.keys().collect();
        assert_eq!(keys, vec![&"about".to_string(), &"home".to_string()]);
    }

    #[test]
    fn test_values() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));
        let values: Vec<&String> = routes.values().collect();
        assert_eq!(values, vec![&"/about".to_string(), &"/home".to_string()]);
    }

    #[test]
    fn test_contains_key() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));

        assert!(routes.contains_key("home"));
        assert!(routes.contains_key("about"));
        assert!(!routes.contains_key("contact"));
    }

    #[test]
    fn test_index() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));

        assert_eq!(routes["home"], "/home".to_string());
        assert_eq!(routes["about"], "/about".to_string());
    }

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn test_index_panics_on_invalid_key() {
        let routes = Routes::from(BTreeMap::new());
        let _ = &routes["invalid"];
    }

    #[test]
    fn test_index_mut() {
        let mut routes = Routes::from(BTreeMap::new());
        routes["home"] = "/new_home".to_string();
        assert_eq!(routes["home"], "/new_home".to_string());
    }

    #[test]
    fn test_iter() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));
        let iter: Vec<(&String, &String)> = routes.iter().collect();
        assert_eq!(
            iter,
            vec![
                (&"about".to_string(), &"/about".to_string()),
                (&"home".to_string(), &"/home".to_string())
            ]
        );
    }

    #[test]
    fn test_into_values() {
        let routes = Routes::from(BTreeMap::from([
            ("home".to_string(), "/home".to_string()),
            ("about".to_string(), "/about".to_string()),
        ]));
        let values: Vec<String> = routes.into_values().collect();
        assert_eq!(values, vec!["/about".to_string(), "/home".to_string()]);
    }
}
