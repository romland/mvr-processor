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

		let mut neighbours: Vec<usize>;// = Vec::with_capacity(100); //vec![0_usize; 0];

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
			
			// The pre-check before calling distance() will actually cut
			// execution time down to 50% (and more in quiet scenarios). It
			// also seems to make execution time a little more predictable.
			// The downside is that it makes epsilon mean something else.
			// if self.euclidean_distance(v, point) <= self.eps {
			if (v.x - point.x).abs() < self.epsilon as i16 && // Math.abs(this.data[i].y - d.y) < this.eps &&
				self.manhattan_distance(v, point) <= self.epsilon
			{
				neighbours.push(i);
			}
		}

		neighbours.to_vec()
	}
/*
This does not work for ... some still unknown reason!

	fn get_neighbours(&mut self, point_ix: usize) -> Vec<usize>
	{
		let neighbours: &mut Vec<usize> = &mut vec![0_usize; 0];

		let point: &MotionVector = &self.data[point_ix];
		let mut i = 0;

		for v in self.data {
			if std::ptr::eq(v, point) {
				continue;
			}
			
			// The pre-check before calling distance() will actually cut
			// execution time down to 50% (and more in quiet scenarios). It
			// also seems to make execution time a little more predictable.
			// The downside is that it makes epsilon mean something else.
			// if self.euclidean_distance(v, point) <= self.eps {
			if (v.x - point.x).abs() < self.epsilon as i16 && // Math.abs(this.data[i].y - d.y) < this.eps &&
				self.manhattan_distance(v, point) <= self.epsilon
			{
				neighbours.push(i);
			}

			i += 1;
		}

		neighbours.to_vec()
	}
*/

	fn expand(&mut self, point_ix: usize, neighbours: &mut Vec<usize>, cluster_ix: usize) {

		self.results[point_ix] = cluster_ix;				// Assign cluster id

		// let mut curr_neighbours: Vec<usize> = Vec::with_capacity(100); //vec![0_usize; 0];
		let mut curr_neighbours: Vec<usize>;// = vec![0_usize; 0];

		let mut curr_point_ix;

		for i in 0..neighbours.len() {
			curr_point_ix = neighbours[i];

			if self.results[curr_point_ix] == 0xffff {
				self.results[curr_point_ix] = 0;			// Visited and marked as noise by default
				curr_neighbours = self.get_neighbours(curr_point_ix);

				if curr_neighbours.len() >= self.min_points {
					self.expand(curr_point_ix, &mut curr_neighbours, cluster_ix);
				}
			}

			if self.results[curr_point_ix] < 1 {
				// Not assigned to a cluster but visited (= 0)
				self.results[curr_point_ix] = cluster_ix;
			}
		}
	}

	// TODO: Consider using a SIMD version?
	#[allow(dead_code)]
	fn euclidean_distance(&self, point1: &MotionVector, point2: &MotionVector) -> f32 {
		(((point2.x - point1.x).pow(2) + (point2.y - point1.y).pow(2)) as f32).sqrt()
	}
	
	#[allow(dead_code)]
	fn manhattan_distance(&self, point1: &MotionVector, point2: &MotionVector) -> f32 {
		((point2.x - point1.x).abs() + (point2.y - point1.y).abs()) as f32
	}
}
