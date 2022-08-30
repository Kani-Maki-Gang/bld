pub trait Queue<T> {
    fn enqueue(&mut self, item: T);
    fn dequeue(&mut self, pids: &[u32]);
    fn contains(&mut self, pid: u32) -> bool;
}
