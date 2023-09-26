pub struct RingBuffer<T, const N: usize> {
    head: usize,
    tail: usize,
    items: usize,
    buffer: [T; N],
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub const fn is_empty(&self) -> bool {
        self.items == 0
    }

    pub const fn is_full(&self) -> bool {
        self.items == N
    }

    pub fn push(&mut self, data: T) {
        if self.is_full() {
            return;
        }

        self.buffer[self.tail] = data;
        self.tail = (self.tail + 1) % N;
        self.items += 1;
    }

    pub fn pop(&mut self) -> T {
        let data = self.buffer[self.head];

        if !self.is_empty() {
            self.head = (self.head + 1) % N;
            self.items -= 1;
        }

        data
    }

    pub fn front(&self) -> T {
        self.buffer[self.head]
    }

    pub const fn len(&self) -> usize {
        self.items
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.items = 0;
    }
}

impl<T: Default + Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self {
            head: 0,
            tail: 0,
            items: 0,
            buffer: [T::default(); N],
        }
    }
}
