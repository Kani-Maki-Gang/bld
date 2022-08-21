pub trait Queue<T> {
    fn enqueue(&mut self, item: T);
    fn refresh(&mut self);
    fn contains(&mut self, pid: u32) -> bool;
}
