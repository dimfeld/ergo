use redis::Script;

// KEYS:
//  1. scheduled items list
//  2. pending items list
// ARGV:
//  1. current time
const ENQUEUE_SCHEDULED_SCRIPT: &str = r##"
            local move_items = redis.call('ZRANGEBYSCORE', KEYS[1], 0, ARGV[1])
            if #move_items == 0 then
                return 0
            end

            redis.call('ZREM', KEYS[1], unpack(move_items))
            redis.call('LPUSH', KEYS[2], unpack(move_items))
            return #move_items
            "##;

pub struct EnqueueScript(redis::Script);

impl EnqueueScript {
    pub fn new() -> Self {
        redis::Script::new(ENQUEUE_SCHEDULED_SCRIPT)
    }
}
