//! Close actual cut boundaries with a skinned flesh rim, muscle and bone core.
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use std::collections::{BTreeMap, BTreeSet};
type Point = [i32; 3];
fn key(p: Vec3) -> Point {
    p.to_array().map(|v| (v * 100_000.).round() as i32)
}
#[derive(Clone, Copy)]
struct Vertex {
    p: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}
struct Builder {
    p: Vec<[f32; 3]>,
    n: Vec<[f32; 3]>,
    color: Vec<[f32; 4]>,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
}
impl Builder {
    fn triangle(&mut self, vertices: [Vertex; 3], colors: [[f32; 4]; 3]) {
        let normal = (vertices[1].p - vertices[0].p)
            .cross(vertices[2].p - vertices[0].p)
            .normalize_or(Vec3::Y);
        for (v, c) in vertices.into_iter().zip(colors) {
            self.p.push(v.p.to_array());
            self.n.push(normal.to_array());
            self.color.push(c);
            self.joints.push(v.joints);
            self.weights.push(v.weights);
        }
    }
    fn mesh(self, skinned: bool) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.p);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.n);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.color);
        if skinned {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                VertexAttributeValues::Uint16x4(self.joints),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, self.weights);
        }
        mesh
    }
}
pub(super) fn mesh(source: &Mesh, mask: u8, keep: bool) -> Mesh {
    let mut out = Builder {
        p: Vec::new(),
        n: Vec::new(),
        color: Vec::new(),
        joints: Vec::new(),
        weights: Vec::new(),
    };
    let skinned = source.contains_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
    if mask == 0 {
        return out.mesh(skinned);
    }
    let Some(VertexAttributeValues::Float32x3(positions)) =
        source.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return out.mesh(skinned);
    };
    let joints = source.attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
    let weights = source.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
    let vertex = |i: usize| Vertex {
        p: Vec3::from_array(positions[i]),
        joints: match joints {
            Some(VertexAttributeValues::Uint16x4(v)) => v[i],
            _ => [0; 4],
        },
        weights: match weights {
            Some(VertexAttributeValues::Float32x4(v)) => v[i],
            _ => [1., 0., 0., 0.],
        },
    };
    let count = source.indices().map_or(positions.len(), Indices::len);
    let at = |i| match source.indices() {
        Some(Indices::U16(v)) => v[i] as usize,
        Some(Indices::U32(v)) => v[i] as usize,
        None => i,
    };
    // Weld position seams for edge adjacency without changing the artist's
    // normals, UVs or skinning. Only edges shared with a removed face are cuts.
    let mut edges: BTreeMap<(Point, Point), (Option<(Vertex, Vertex)>, bool)> = BTreeMap::new();
    for i in (0..count.saturating_sub(2)).step_by(3) {
        let face = [vertex(at(i)), vertex(at(i + 1)), vertex(at(i + 2))];
        let center = (face[0].p + face[1].p + face[2].p) / 3.;
        let retained = (super::body::region(center) & mask != 0) == keep;
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let (ka, kb) = (key(a.p), key(b.p));
            if ka == kb {
                continue;
            }
            let edge = edges
                .entry(if ka < kb { (ka, kb) } else { (kb, ka) })
                .or_default();
            if retained {
                edge.0 = Some((a, b));
            } else {
                edge.1 = true;
            }
        }
    }
    let boundary: Vec<_> = edges
        .into_values()
        .filter_map(|(edge, removed)| edge.filter(|_| removed))
        .collect();
    let mut adjacency: BTreeMap<Point, Vec<Point>> = BTreeMap::new();
    let mut vertices = BTreeMap::new();
    for &(a, b) in &boundary {
        adjacency.entry(key(a.p)).or_default().push(key(b.p));
        adjacency.entry(key(b.p)).or_default().push(key(a.p));
        vertices.insert(key(a.p), a);
        vertices.insert(key(b.p), b);
    }
    let mut visited = BTreeSet::new();
    let mut centers = BTreeMap::new();
    for &start in vertices.keys() {
        if visited.contains(&start) {
            continue;
        }
        let mut pending = vec![start];
        let mut group = Vec::new();
        while let Some(at) = pending.pop() {
            if !visited.insert(at) {
                continue;
            }
            group.push(at);
            if let Some(neighbors) = adjacency.get(&at) {
                pending.extend(neighbors.iter().copied());
            }
        }
        let p = group.iter().map(|at| vertices[at].p).sum::<Vec3>() / group.len() as f32;
        let mut influences: BTreeMap<u16, f32> = BTreeMap::new();
        for at in &group {
            let v = vertices[at];
            for (joint, weight) in v.joints.into_iter().zip(v.weights) {
                *influences.entry(joint).or_default() += weight;
            }
        }
        let mut influences: Vec<_> = influences.into_iter().collect();
        influences.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut center = Vertex {
            p,
            joints: [0; 4],
            weights: [0.; 4],
        };
        let total: f32 = influences.iter().take(4).map(|(_, w)| w).sum();
        for (i, (joint, w)) in influences.into_iter().take(4).enumerate() {
            center.joints[i] = joint;
            center.weights[i] = w / total.max(1e-8);
        }
        for at in group {
            centers.insert(at, center);
        }
    }
    let rim = [0.24, 0.003, 0.009, 1.];
    let muscle = [0.65, 0.035, 0.055, 1.];
    let bone = [0.88, 0.62, 0.40, 1.];
    for (a, b) in boundary {
        let center = centers[&key(a.p)];
        let mid = |v: Vertex, t: f32| Vertex {
            p: center.p.lerp(v.p, t),
            ..v
        };
        let (am, bm, ai, bi) = (mid(a, 0.78), mid(b, 0.78), mid(a, 0.20), mid(b, 0.20));
        out.triangle([b, a, am], [rim, rim, muscle]);
        out.triangle([b, am, bm], [rim, muscle, muscle]);
        out.triangle([bm, am, ai], [muscle, muscle, bone]);
        out.triangle([bm, ai, bi], [muscle, bone, bone]);
        out.triangle([bi, ai, center], [bone, bone, bone]);
    }
    out.mesh(skinned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_cut_closes_the_actual_boundary_and_carries_skinning_and_flesh_colors() {
        let mut source = Mesh::from(Cuboid::new(0.4, 0.4, 0.4));
        // Straddle the leg cutoff at y=.66; the retained and detached halves
        // have UV/normal seams, as do the production GLBs.
        if let Some(VertexAttributeValues::Float32x3(p)) =
            source.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for p in p {
                p[0] += 0.4;
                p[1] += 0.66;
            }
        }
        let count = source.count_vertices();
        source.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![[3, 0, 0, 0]; count]),
        );
        source.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; count]);
        for keep in [false, true] {
            let cap = mesh(&source, 4, keep);
            assert!(cap.count_vertices() >= 30);
            let Some(VertexAttributeValues::Float32x3(p)) = cap.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            assert!(p.iter().flatten().all(|x| x.is_finite()));
            let mut edges = BTreeMap::new();
            let mut add = |triangle: [Vec3; 3]| {
                for (a, b) in [
                    (triangle[0], triangle[1]),
                    (triangle[1], triangle[2]),
                    (triangle[2], triangle[0]),
                ] {
                    let (a, b) = (key(a), key(b));
                    if a != b {
                        *edges
                            .entry(if a < b { (a, b) } else { (b, a) })
                            .or_insert(0_usize) += 1;
                    }
                }
            };
            let Some(VertexAttributeValues::Float32x3(original)) =
                source.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!()
            };
            let indices: Vec<_> = source.indices().unwrap().iter().collect();
            for tri in indices.chunks_exact(3) {
                let triangle = [tri[0], tri[1], tri[2]].map(|i| Vec3::from_array(original[i]));
                let center = (triangle[0] + triangle[1] + triangle[2]) / 3.;
                if (super::super::body::region(center) & 4 != 0) == keep {
                    add(triangle);
                }
            }
            for tri in p.chunks_exact(3) {
                add([
                    Vec3::from_array(tri[0]),
                    Vec3::from_array(tri[1]),
                    Vec3::from_array(tri[2]),
                ]);
            }
            assert!(
                edges.values().all(|n| *n == 2),
                "the cut must have no open edges"
            );
            let Some(VertexAttributeValues::Float32x4(w)) =
                cap.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
            else {
                panic!()
            };
            assert!(w.iter().all(|w| (w.iter().sum::<f32>() - 1.).abs() < 1e-6));
            let Some(VertexAttributeValues::Float32x4(colors)) =
                cap.attribute(Mesh::ATTRIBUTE_COLOR)
            else {
                panic!()
            };
            assert!(colors.iter().any(|c| c[0] > 0.8 && c[1] > 0.5));
            assert!(colors.iter().any(|c| c[0] > 0.5 && c[1] < 0.1));
        }
        assert_eq!(mesh(&source, 0, false).count_vertices(), 0);
    }
}
