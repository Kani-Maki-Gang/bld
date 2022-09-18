pub trait Queue<T> {
    fn enqueue(&mut self, item: T) -> anyhow::Result<()>;
    fn dequeue(&mut self, pids: u32) -> anyhow::Result<()>;
    fn contains(&mut self, pid: u32) -> bool;
}
