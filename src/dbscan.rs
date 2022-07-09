use crate::mvrprocessor::MotionVector;

pub struct DbScan<'a> {
	pub epsilon: f32,
	pub min_points: usize,
	pub data: &'a Vec<MotionVector>,
	pub results: &'a mut Vec<usize>,
}

impl<'a> DbScan<'_> {
	pub fn run(&mut self)
	{
		let mut next_ix: usize = 0;

		let mut neighbours: Vec<usize>;

		for i in 0..self.data.len() {
			if self.results[i] != 0xffff {
				continue;
			}
	
			self.results[i] = 0;
			neighbours = self.get_neighbours(i);

			if neighbours.len() >= self.min_points {
				next_ix += 1;
				self.expand(i, &mut neighbours, next_ix);
			}

		}
	}

	fn get_neighbours(&mut self, point_ix: usize) -> Vec<usize>
	{
		let neighbours: &mut Vec<usize> = &mut vec![0_usize; 0];

		let point: &MotionVector = &self.data[point_ix];
		let mut v;

		for i in 0..self.data.len() {
			if i == point_ix {
				continue;
			}

			v = &self.data[i];
			
			// The pre-check before calling x_distance() will actually cut
			// execution time down to 50% (and more in quiet scenarios). It
			// also makes execution time a little more predictable. The (big)
			// downside is that it makes epsilon mean something else.
			if (v.x - point.x).abs() < self.epsilon as i16 && 
			   self.manhattan_distance(v, point) <= self.epsilon {
				neighbours.push(i);
			}
		}

		neighbours.to_vec()
	}

	fn expand(&mut self, point_ix: usize, neighbours: &mut Vec<usize>, cluster_ix: usize) {

		// Assign cluster id (which is just an index)
		self.results[point_ix] = cluster_ix;

		let mut curr_neighbours: Vec<usize>;

		let mut curr_point_ix;

		for i in 0..neighbours.len() {
			curr_point_ix = neighbours[i];

			if self.results[curr_point_ix] == 0xffff {
				// Default: Point visited and marked as noise
				self.results[curr_point_ix] = 0;
				curr_neighbours = self.get_neighbours(curr_point_ix);

				if curr_neighbours.len() >= self.min_points {
					self.expand(curr_point_ix, &mut curr_neighbours, cluster_ix);
				}
			}

			if self.results[curr_point_ix] < 1 {
				// Point not assigned to a cluster but visited (= 0)
				self.results[curr_point_ix] = cluster_ix;
			}
		}
	}

	// TODO: Consider using some kind of a SIMD version? How to do it in Rust?
	// Note: As it is right now, I am actually quite fine with using manhattan distance, 
	//       which would not benefit awesomely from SIMD.
	#[allow(dead_code)]
	fn euclidean_distance(&self, point1: &MotionVector, point2: &MotionVector) -> f32 {
		(((point2.x - point1.x).pow(2) + (point2.y - point1.y).pow(2)) as f32).sqrt()
	}
	
	#[allow(dead_code)]
	fn manhattan_distance(&self, point1: &MotionVector, point2: &MotionVector) -> f32 {
		((point2.x - point1.x).abs() + (point2.y - point1.y).abs()) as f32
	}
}
