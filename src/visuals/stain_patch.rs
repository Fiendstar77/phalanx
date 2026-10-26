//! Clip a splash to the receiver's triangles. No view-dependent UV projection.
use bevy::{
    asset::RenderAssetUsages,
    math::Affine3A,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
#[derive(Clone, Copy)]
struct Vertex {
    p: Vec3,
    q: Vec3,
}
pub(super) fn clipped(
    source: &Mesh,
    world: Affine3A,
    point: Vec3,
    normal: Vec3,
    radius: f32,
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let Some(VertexAttributeValues::Float32x3(points)) = source.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return empty();
    };
    let normal = normal.normalize_or(Vec3::Y);
    let up = (Vec3::Y - normal * normal.y).normalize_or(Vec3::Z);
    let tangent = up.cross(normal).normalize_or(Vec3::X);
    let inverse = world.inverse();
    let count = source.indices().map_or(points.len(), Indices::len);
    let at = |i| match source.indices() {
        Some(Indices::U16(v)) => v[i] as usize,
        Some(Indices::U32(v)) => v[i] as usize,
        None => i,
    };
    for i in (0..count.saturating_sub(2)).step_by(3) {
        let face = [at(i), at(i + 1), at(i + 2)]
            .map(|j| world.transform_point3(Vec3::from_array(points[j])));
        let n = (face[1] - face[0])
            .cross(face[2] - face[0])
            .normalize_or_zero();
        if n.dot(normal) < 0.15 {
            continue;
        }
        let mut polygon: Vec<_> = face
            .into_iter()
            .map(|p| {
                let d = p - point;
                Vertex {
                    p,
                    q: Vec3::new(d.dot(tangent), d.dot(up), d.dot(normal)),
                }
            })
            .collect();
        for (axis, limit) in [(0, radius), (1, radius), (2, radius * 0.65 + 0.008)] {
            for sign in [-1., 1.] {
                if polygon.is_empty() {
                    break;
                }
                let mut next = Vec::new();
                let mut a = *polygon.last().unwrap();
                for &b in &polygon {
                    let da = limit - a.q[axis] * sign;
                    let db = limit - b.q[axis] * sign;
                    if (da >= 0.) != (db >= 0.) {
                        let t = da / (da - db);
                        next.push(Vertex {
                            p: a.p.lerp(b.p, t),
                            q: a.q.lerp(b.q, t),
                        });
                    }
                    if db >= 0. {
                        next.push(b);
                    }
                    a = b;
                }
                polygon = next;
            }
        }
        if polygon.len() < 3 {
            continue;
        }
        let local_normal = world.matrix3.transpose().mul_vec3(n).normalize_or(Vec3::Y);
        for j in 1..polygon.len() - 1 {
            let triangle = [polygon[0], polygon[j], polygon[j + 1]];
            if (triangle[1].p - triangle[0].p)
                .cross(triangle[2].p - triangle[0].p)
                .length_squared()
                < 1e-12
            {
                continue;
            }
            for v in triangle {
                positions.push(inverse.transform_point3(v.p + n * 0.002).to_array());
                normals.push(local_normal.to_array());
                uv.push([
                    (v.q.x / radius * 0.5 + 0.5).clamp(0., 1.),
                    (0.5 - v.q.y / radius * 0.5).clamp(0., 1.),
                ]);
            }
        }
    }
    let mut mesh = empty();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    mesh
}
fn empty() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_stay_on_the_receiver_under_rotation_and_nonuniform_scale() {
        let source = Mesh::from(Cuboid::new(1., 1., 1.));
        let world = Transform::from_xyz(3., 2., -4.)
            .with_rotation(Quat::from_rotation_y(0.6) * Quat::from_rotation_z(0.3))
            .with_scale(Vec3::new(0.2, 3., 1.4))
            .compute_affine();
        let point = world.transform_point3(Vec3::new(-0.5, 0.1, 0.));
        let normal = world
            .inverse()
            .matrix3
            .transpose()
            .mul_vec3(Vec3::NEG_X)
            .normalize();
        let patch = clipped(&source, world, point, normal, 0.24);
        assert!(patch.count_vertices() >= 6);
        let Some(VertexAttributeValues::Float32x3(p)) = patch.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!()
        };
        for p in p {
            let p = world.transform_point3(Vec3::from_array(*p));
            assert!(
                ((p - point).dot(normal) - 0.002).abs() < 1e-5,
                "stain floated off the receiving triangle"
            );
            assert!(p.distance(point) < 0.35);
        }
        let Some(VertexAttributeValues::Float32x2(uv)) = patch.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!()
        };
        assert!(
            uv.iter()
                .flatten()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        );
    }
}
