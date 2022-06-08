//! GraphQL implementation for [`Iterator`].

use crate::{graphql, resolve, ExecutionResult, Executor, Selection};

/*
pub fn resolve_list<'t, T, S, Info, Ctx, I>(
    iter: I,
    selection_set: Option<&[Selection<'_, S>]>,
    info: &Info,
    executor: &Executor<Ctx, S>,
) -> ExecutionResult<S>
where
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: resolve::Value<Info, Ctx, S> + ?Sized + 't,
    Info: ?Sized,
    Ctx: ?Sized,
{
    let is_non_null = executor
        .current_type_new()
        .list_contents()
        .ok_or("Iterating over non-list type")?
        .is_non_null();

    let mut values = Vec::with_capacity(iter.len());
    for v in iter {
        let val = v.resolve_value(selection_set, info, executor)?;
        if is_non_null && val.is_null() {
            return Err("Resolved `null` on non-null type".into());
        }
        values.push(val);
    }
    Ok(graphql::Value::list(values))
}

pub async fn resolve_list_async<'a, 't, T, S, Info, Ctx, I>(
    iter: I,
    selection_set: Option<&[Selection<'_, S>]>,
    info: &'a Info,
    executor: &'a Executor<'a, 'a, Ctx, S>,
) -> ExecutionResult<S>
where
    I: Iterator<Item = &'t T> + ExactSizeIterator,
    T: resolve::ValueAsync<Info, Ctx, S> + ?Sized + 't,
    Info: ?Sized,
    Ctx: ?Sized,
{
    use futures::stream::{FuturesOrdered, StreamExt as _};

    let is_non_null = executor
        .current_type_new()
        .list_contents()
        .ok_or("Iterating over non-list type")?
        .is_non_null();

    let mut futs = iter
        .map(|v| async move { v.resolve_value_async(selection_set, info, executor).await })
        .collect::<FuturesOrdered<_>>();

    let mut values = Vec::with_capacity(futs.len());
    while let Some(res) = futs.next().await {
        let val = res?;
        if is_non_null && val.is_null() {
            return Err("Resolved `null` on non-null type".into());
        }
        values.push(val);
    }
    Ok(graphql::Value::list(values))
}
*/
