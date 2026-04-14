use redis::AsyncCommands;

pub async fn check_and_increment(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    limit: u32,
    window_secs: u64,
) -> Result<bool, redis::RedisError> {
    let count: u32 = redis.get(key).await.unwrap_or(0);
    if count >= limit {
        return Ok(false);
    }
    let _: () = redis.incr(key, 1).await?;
    if count == 0 {
        let _: () = redis.expire(key, window_secs as i64).await?;
    }
    Ok(true)
}

pub async fn is_blocked(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
) -> bool {
    redis.exists(key).await.unwrap_or(false)
}

pub async fn set_block(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    seconds: u64,
) -> Result<(), redis::RedisError> {
    let _: () = redis.set_ex(key, "blocked", seconds as u64).await?;
    Ok(())
}
