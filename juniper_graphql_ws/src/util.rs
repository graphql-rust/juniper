use serde::{Deserialize, Deserializer};

/// Deserializes `null`able value by placing the [`Default`] value instead of `null`.
pub(crate) fn default_for_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}
