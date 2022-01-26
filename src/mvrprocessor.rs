use std::net::{TcpStream};
use bufstream::BufStream;
use std::sync::{Arc,RwLock};
use std::sync::mpsc::{Sender};
use byteorder::{ByteOrder, LittleEndian};
use std::f32::consts::PI;
use std::io::prelude::*;
use std::time::{Instant,SystemTime,UNIX_EPOCH};
use std::collections::HashMap;
use crate::dbscan::DbScan;
use serde::{Deserialize, Serialize};

#[derive(Clone,Copy,Debug)]
#[derive(Serialize, Deserialize)]
pub struct MotionVector {
    pub dx: i8,
    pub dy: i8,
	pub sad: u16,
	pub x : i16,
	pub y : i16,
	pub dir : f32,
	pub mag : f32,
	pub org_x : i16,
	pub org_y : i16,
}

impl MotionVector {
    pub fn new() -> MotionVector {
        MotionVector {
			dx: 0,
			dy: 0,
			sad: 0,
			x: -1,
			y: -1,
			dir: 0.0,
			mag: 0.0,
			org_x : 0,
			org_y : 0,
		}
    }
}

#[derive(Clone,Debug)]
#[derive(Serialize, Deserialize)]
pub struct Cluster {
	id: usize,
	points: Vec<MotionVector>,
	dir : f32,
	mag : f32,
	bbox : [i16; 4],
	within: bool,
	birth : u128,
	age: u128,
	active: u128,
	size: usize,
}

impl Cluster {
	#[allow(dead_code)]
    pub fn new() -> Cluster {
        Cluster {
			id: 0,
			points: vec![],
			dir: 0.0,
			mag: 0.0,
			bbox: [1000, 0, 0, 1000],
			within: false,
			birth: 0,
			age: 0,
			active: 0,
			size: 0,
		}
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct FrameInfo {
	nullFrame : bool,		// false whether we for some reason skipped processing this frame
	totalMagnitude : i32,	// 0,	 total magnitude of all vectors in this frame
	candidates : i32,		// 0,	 number of vectors/blocks that were deemed active in this frame
	ignoredVectors : i32,	// 0,	 number of vectors that we found in an ignored area
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct ClusterMessage {
	clusters: Vec<Cluster>,
	history: Vec<Cluster>,
	frameInfo: FrameInfo,
}


#[allow(unused_variables)]
pub fn handle_raw_mvr_connection(stream: &mut BufStream<TcpStream>, chan: Sender<String>, arc: Arc<RwLock<Vec<String>>>)
{
	const VECTORS_WIDTH: usize = 121;	// 1920
	const VECTORS_HEIGHT: usize = 68;	// 1080
	const VECTOR_COUNT: usize = VECTORS_WIDTH * VECTORS_HEIGHT;
	const BUFSIZE: usize = VECTOR_COUNT * 4;
	const DISCARD_CLUSTERS_AFTER: u128 = 2000;

	let now = Instant::now();

	let mut buffer = [0; BUFSIZE];
	let mut vectors:Vec<MotionVector> = vec![MotionVector::new(); VECTOR_COUNT];
	let mut candidates:Vec<MotionVector> = vec![];
	let mut frame_counter = 0;
	let mut total_mag: f32;
	let mut history: Vec<Cluster> = vec![];
	let mut epoch;
	let mut frame_start;
	let mut last_history_id = 0;

	loop {
        stream.read_exact(&mut buffer).unwrap(); //TODO: non-blocking read
		
		if (now.elapsed().as_millis()) < 1000 {
			println!(r#"{{"err":"Just started; skipping frame"}}"#);
			continue;
		}

		epoch = SystemTime::now()
        	.duration_since(UNIX_EPOCH)
        	.expect("Time was weird");
		frame_start = epoch.as_millis();

		total_mag = 0.0;
		frame_counter += 1;

		candidates.clear();

		// let mut debug_ascii: Vec<u8> = vec![b' '; VECTOR_COUNT];

		for mv in (0..BUFSIZE).step_by(4) {
			let index = mv / 4;

			vectors[index].dx = buffer[mv + 0] as i8;
			vectors[index].dy = buffer[mv + 1] as i8;

			// TODO: what is it, actually: 1) signed/unsigned? 2) little/big endian?
			vectors[index].sad = LittleEndian::read_u16(&buffer[mv + 2..mv + 4]);

			if vectors[index].x == -1 {
				vectors[index].x = (index % VECTORS_WIDTH) as i16;
				vectors[index].y = (index / VECTORS_WIDTH) as i16;
			}
			
			vectors[index].dir = (vectors[index].dy as f32).atan2(-(vectors[index].dx) as f32) * 180.0 / PI + 180.0;
			vectors[index].mag = (
				(
					(vectors[index].dx as i32) * (vectors[index].dx as i32) +
					(vectors[index].dy as i32) * (vectors[index].dy as i32)
				) as f32).sqrt();

			// TODO: This 'sad' check is just a test to get rid of noise in low-light conditions...
			if vectors[index].mag >= 2.0 && vectors[index].sad > 250 {
				total_mag += vectors[index].mag;		// TODO: to include mag of all or just ones that are deemed active?
				candidates.push(vectors[index].clone());
				// debug_ascii[index] = b'*';
				// println!("YES {}", vectors[index].sad);
			} else {
				// debug_ascii[index] = b'.';
				// println!("NO  {}", vectors[index].sad);
			}
		}
		
		// This output looks good too and it's not reduce (reproduced without that code),
		// ...which means it's dbscan!
		/*
		for i in (0..debug_ascii.len()).step_by(VECTORS_WIDTH) {
			let line = match std::str::from_utf8(&debug_ascii[i..i+VECTORS_WIDTH]) {
				Ok(v) => v,
				Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
			};
			println!("{}", line);
		}
		*/

		// reduce!
		let (reduced, factor) = match reduce_candidates(&mut candidates) {
			None => (false, 1),
			Some((new_candidates, factor)) => {
				candidates = new_candidates;
				(true, factor)
			}
		};

		let mut results: Vec<usize> = vec![0x000000000000ffff_usize; candidates.len()];
		let frame = &mut DbScan {
			epsilon: 2.0,
			min_points: 4,
			data: &candidates,
			results: &mut results,
		};
		frame.run();

		// debug_associate_result_candidates(&results, &candidates);

		let clusters = refine_clusters(&mut candidates, &results, reduced, &mut history, &frame_start, &mut last_history_id);

		if history.len() > 0 {
			temporal_expiration(&mut history, &frame_start, DISCARD_CLUSTERS_AFTER);
		}

		// let ser_cost = Instant::now();
		// TODO: Can I get rid of this .clone() somehow?
		let msg = ClusterMessage {
			clusters: clusters,
			history: history.clone(),
			frameInfo: FrameInfo {
				totalMagnitude: total_mag as i32,
				candidates: candidates.len() as i32,
    			nullFrame: false,		// TODO see definition
    			ignoredVectors: 0,		// TODO see definition
			}
		};

		let json = serde_json::to_string(&msg).unwrap();
		// println!("JSON+write {}ms", ser_cost.elapsed().as_millis());

		// This is crucial if deploying to Raspi
		println!("{}", json);
    }
}


#[allow(dead_code)]
fn debug_associate_result_candidates(results: &Vec<usize>, candidates: &Vec<MotionVector>)
{
	let max_clusters = match results.iter().max() {
		Some(max) => max,
		None => &0_usize
	};

	// Clusters[ (x,y), (x,y), ... ]
	let mut debug_vec: Vec<Vec<(usize,usize)>> = vec![vec![]; *max_clusters + 1];

	for i in 0..results.len() {
		if results[i] == 0 {
			continue;
		}

		if debug_vec[results[i]].len() > 0 {
			debug_vec[results[i]].push( 
				(candidates[i].org_x as usize, candidates[i].org_y as usize)
			);
		} else {
			debug_vec[results[i]] = vec![ 
				(candidates[i].org_x as usize, candidates[i].org_y as usize) 
			];
		}
	}

	for i in 0..debug_vec.len() {
		let mut minx = 1000;
		let mut maxx = 0;
		let mut miny = 1000;
		let mut maxy = 0;

		if debug_vec[i].len() > 0 {
			for j in 0..debug_vec[i].len() {
				if debug_vec[i][j].0 < minx { minx = debug_vec[i][j].0; }
				if debug_vec[i][j].0 > maxx { maxx = debug_vec[i][j].0; }

				if debug_vec[i][j].1 < miny { miny = debug_vec[i][j].1; }
				if debug_vec[i][j].1 > maxy { maxy = debug_vec[i][j].1; }
			}
		} else {
			minx = 0;
			maxx = 0;
			miny = 0;
			maxy = 0;
		}

		println!("debug cluster {}; size: {}x{} {} points: {:?}", 
			i, 
			maxx - minx,
			maxy - miny,
			debug_vec[i].len(), 
			debug_vec[i]
		);
	}
}


// The idea: If we have a lot of candidates: Shrink the dataset by reducing 'resolution'
// remove every Nth and divide the coordinate of vector by N
// let's say, if it is above 200 (nee 400) points, get it down to that...
fn reduce_candidates(candidates: &mut Vec<MotionVector>) -> Option<(Vec<MotionVector>, usize)>
{
	let reduction_factor;
	let target_candidates = 100;

	// was * 1.25, but I am less picky about filtering out in pre-stage now...
	if candidates.len() as f32 > (target_candidates as f32 * 1.25) {
		reduction_factor = (candidates.len() / target_candidates) as usize;
		let mut reduced_candidates: Vec<MotionVector> = vec![];

		for i in (0..candidates.len()).step_by(reduction_factor) {
			candidates[i].org_x = candidates[i].x;
			candidates[i].org_y = candidates[i].y;
			candidates[i].x = candidates[i].x / reduction_factor as i16;
			candidates[i].y = candidates[i].y / reduction_factor as i16;

			reduced_candidates.push(candidates[i]);
		}

		return Some((reduced_candidates, reduction_factor));
	}

	None
}


/*
 * On clustering:
 * Returns array 'results' (of same size as candidates):
 *
 *	if results[i] is 0 = noise
 *	oterhwise results[i] = a cluster id
 *		and 'i' = index of the candidate in 'candidates'
 *
 *	we then take candidates[i] and throw that into a grouped
 *	collection. Ie. cluster[cluster-id] = [ candidates... ]
 */
fn refine_clusters(
	candidates: &mut Vec<MotionVector>, results: &Vec<usize>, reduced: bool, 
	history: &mut Vec<Cluster>, now: &u128, last_history_id: &mut usize) -> Vec<Cluster>
{
	let mut cluster: &mut Cluster;
	let mut clusters_map: HashMap<usize, Cluster> = HashMap::new();

	// A result refers to an index in candidates
	for i in 0..results.len() {
		if results[i] == 0 {
			// cluster 0 is noise
			continue;
		}

		cluster = clusters_map.entry(results[i]).or_insert(Cluster {
			id: 0,
			birth: *now,
			points: vec![],
			dir: 0.0,
			mag: 0.0,
			bbox: [1000,0,0,1000],
			within: false,
			age: 0,
			active: 0,
			size: 0,
		});

		if reduced {
			candidates[i].x = candidates[i].org_x;
			candidates[i].y = candidates[i].org_y;
		}

		cluster.points.push(candidates[i]);

		// Bounding box
		if candidates[i].y < cluster.bbox[0] {
			cluster.bbox[0] = candidates[i].y;
		}

		if candidates[i].x > cluster.bbox[1] {
			cluster.bbox[1] = candidates[i].x;
		}

		if candidates[i].y > cluster.bbox[2] {
			cluster.bbox[2] = candidates[i].y;
		}

		if candidates[i].x < cluster.bbox[3] {
			cluster.bbox[3] = candidates[i].x;
		}

		cluster.dir += candidates[i].dir;
		cluster.mag += candidates[i].mag;
	}

	let mut clusters: Vec<Cluster> = clusters_map.into_values().collect();

	for k in 0..clusters.len() {
		if is_within(k, &clusters) {
			clusters[k].within = true;
		} else {
			// Cluster is not discarded
			track_temporal(history, &mut clusters[k], now, last_history_id);
		}

		clusters[k].dir /= clusters[k].points.len() as f32;
		clusters[k].mag /= clusters[k].points.len() as f32;
	}

	clusters
}

fn is_within(my_index: usize, others: &Vec<Cluster>) -> bool
{
	let cluster = &others[my_index];

	for k in 0..others.len() {
		if k == my_index {
			continue;
		}

		if cluster.bbox[0] >= others[k].bbox[0] 			// >= top
	  		&& cluster.bbox[2] <= others[k].bbox[2] 		// <= bottom
	  		&& cluster.bbox[3] >= others[k].bbox[3]			// >= left
	  		&& cluster.bbox[1] <= others[k].bbox[1] {		// <= right
			return true;
		}
	}

	false
}

fn track_temporal(history: &mut Vec<Cluster>, cluster: &mut Cluster, now: &u128, last_history_id: &mut usize)
{
	match overlaps_any(cluster, history) {
		Some(overlapping_index) => {
			let overlapping = &mut history[overlapping_index];

			// println!("UPDATING CLUSTER ID {}", overlapping.id);

			// update cluster in history
			cluster.age = now - overlapping.birth;
			overlapping.active = *now;
			overlapping.age = cluster.age;

			overlapping.bbox = cluster.bbox.clone();
			overlapping.size = cluster.points.len();

			overlapping.points = cluster.points.clone();
			overlapping.mag = cluster.mag;
			overlapping.dir = cluster.dir;
		},
		None => {
			// add new cluster to history
			*last_history_id += 1;

			// println!("NEW CLUSTER ID: {} -- history size: {}", &last_history_id, history.len());

			history.push(Cluster {
				id : *last_history_id,
				age : 0,
				active : *now,
				birth : *now,
				within: cluster.within,
				bbox : cluster.bbox.clone(),
				size : cluster.points.len(),

				points : cluster.points.clone(),
				mag : cluster.mag,
				dir : cluster.dir
			});
		}
	}
}

fn temporal_expiration(history: &mut Vec<Cluster>, now: &u128, expire_after: u128)
{
	history.retain(|v| {
		(now - v.active) <= expire_after
	});
}

fn overlaps_any(c: &Cluster, history: &Vec<Cluster>) -> Option<usize>
{
	for i in 0..history.len() {
		if overlaps(c, &history[i]) {
			return Some(i);
		}
	}

	None
}

fn overlaps(c1: &Cluster, c2: &Cluster) -> bool
{
	if c1.bbox[1] < c2.bbox[3] {
		return false;
	}

	if c2.bbox[1] < c1.bbox[3] {
		return false;
	}

	if c1.bbox[2] < c2.bbox[0] {
		return false;
	}

	if c2.bbox[2] < c1.bbox[0] {
		return false;
	}

	true
}


/*
// overlapping is the cluster in history
trackTemporal(cluster, now)
{
	let overlapping = this.overlapsAny(cluster, this.history);
	if(overlapping !== false) {
		// update cluster in history
		cluster.age = now - overlapping.birth;
		overlapping.active = now;
		overlapping.age = cluster.age;

		// Do I want to update the history box? Let's see...
		// Do it only if we are more dense (and often bigger) than the one stored...
//			if(cluster.points.length > overlapping.size) {
			overlapping.bbox = [...cluster.bbox];
			overlapping.size = cluster.points.length;

			overlapping.points = [...cluster.points];
			overlapping.mag = cluster.mag;
			overlapping.dir = cluster.dir;
//			}
	} else {
		// add new cluster to history
		this.history.push({
			id : this.historyClusterId++,
			age : 0,
			active : now,
			birth : now,
			bbox : [...cluster.bbox],
			size : cluster.points.length,

			points : [...cluster.points],
			mag : cluster.mag,
			dir : cluster.dir
		});
	}
}


// discardInactiveAfter
// Expire ones that have had no activity for expireAfter ms
temporalExpiration(now)
{
	// TODO: Move out of this scope
	const expireAfter = this.conf.get("discardInactiveAfter");

	for(let i = this.history.length - 1; i >= 0; i--) {
		if((now - this.history[i].active) > expireAfter) {
			this.history.splice(i, 1);
		}
	}
}

overlapsAny(c, cAll)
{
	for(let i = 0; i < cAll.length; i++) {
		if(this.overlaps(c, cAll[i])) {
			return cAll[i];
		}
	}

	return false;
}

overlaps(c1, c2)
{
	if (c1.bbox[1] < c2.bbox[3]) return false;
	if (c2.bbox[1] < c1.bbox[3]) return false;

	if (c1.bbox[2] < c2.bbox[0]) return false;
	if (c2.bbox[2] < c1.bbox[0]) return false;

	return true;
}
*/