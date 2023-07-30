#[derive(Clone)]
pub struct NodeReference {
    pub id: usize,
    pub distance: f32,
}

pub struct BestResults {
    results: Vec<NodeReference>,
    worst_result_index: usize,
    worst_distance: f32,
    size: usize,
}

impl BestResults {
    pub fn new(size: usize) -> BestResults {
        BestResults {
            results: Vec::with_capacity(size),
            worst_result_index: 0,
            worst_distance: 0.0,
            size,
        }
    }
    pub fn insert(&mut self, r: NodeReference) -> bool {
        if self.results.len() < self.size {
            if self.contains_id(r.id) {
                return false;
            }
            self.results.push(r);
            if self.results.len() == self.size {
                // Transition to the 'full' state.
                self.update_worst()
            }
            return true;
        }
        if r.distance < self.worst_distance {
            if self.contains_id(r.id) {
                return false;
            }
            self.results[self.worst_result_index] = r;
            self.update_worst();
            return true;
        }
        false
    }

    fn contains_id(&self, id: usize) -> bool {
        self.results.iter().any(|x| x.id == id)
    }

    pub fn sort(&mut self) {
        if self.results.len() == 0 {
            return;
        }
        self.results
            .sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        self.worst_result_index = self.results.len() - 1;
        self.worst_distance = self.results[self.results.len() - 1].distance;
    }

    pub fn results(&self) -> &Vec<NodeReference> {
        &self.results
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn clear(&mut self) {
        self.results.clear();
    }

    pub fn worst_distance(&self) -> f32 {
        return self.worst_distance;
    }

    fn update_worst(&mut self) {
        self.worst_result_index = 0;
        self.worst_distance = self.results[0].distance;
        for i in 1..self.results.len() {
            let r = &self.results[i];
            if r.distance > self.worst_distance {
                self.worst_distance = r.distance;
                self.worst_result_index = i;
            }
        }
    }
}
