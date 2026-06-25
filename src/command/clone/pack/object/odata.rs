#[derive(Debug)]
pub enum ObjectData {
    Base(Vec<u8>),
    OfsDelta {
        base_distance: usize,
        delta_data: Vec<u8>,
    },
    RefDelta {
        sha: [u8; 20],
        delta_data: Vec<u8>,
    },
}
