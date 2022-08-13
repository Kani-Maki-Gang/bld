pub trait Queue<T> {
    fn enqueue(&mut self, item: T);
    fn refresh(&mut self);
    fn find(&mut self, id: u32) -> Option<T>;
}
