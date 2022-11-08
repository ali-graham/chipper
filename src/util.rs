pub(crate) fn boxed_array<T: Default>(size: usize) -> Box<[T]> {
    let mut vec: Vec<T> = Vec::with_capacity(size);
    for _i in 0..vec.capacity() {
        vec.push(T::default());
    }
    vec.into_boxed_slice()
}
