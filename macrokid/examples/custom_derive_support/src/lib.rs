pub trait AssocDemo {
    type Output;
    const COUNT: usize;
    fn get(&self) -> Self::Output;
}

