//!

use std::collections::HashMap;

pub struct Duplicate<T> {
    pub name: String,
    pub spanned: Vec<T>,
}

impl<T> Duplicate<T> {
    pub fn find_by_key<'a, F>(items: &'a [T], name: F) -> Option<Vec<Duplicate<&'a T>>>
    where
        T: 'a,
        F: Fn(&'a T) -> &'a str,
    {
        let mut mapping: HashMap<&str, Vec<&T>> = HashMap::with_capacity(items.len());

        for item in items {
            if let Some(vals) = mapping.get_mut(name(item)) {
                vals.push(item);
            } else {
                mapping.insert(name(item), vec![item]);
            }
        }

        let duplicates = mapping
            .into_iter()
            .filter_map(|(k, v)| {
                if v.len() != 1 {
                    Some(Duplicate {
                        name: k.to_string(),
                        spanned: v,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !duplicates.is_empty() {
            Some(duplicates)
        } else {
            None
        }
    }
}
