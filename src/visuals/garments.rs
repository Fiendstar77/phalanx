//! Clothes follow the body's exact weights; covered skin is removed beneath them.
use super::*;
use bevy::mesh::{Indices, VertexAttributeValues};
pub(super) fn layer(p: Vec3) -> Option<u8> {
    if p.y > 0.70 && p.y < 1.22 && (p.x.abs() < 0.32 || (p.y > 1.0 && p.x.abs() < 0.52)) {
        Some(0)
    } else if p.y > -0.02 && p.y <= 0.74 {
        Some(1)
    } else {
        None
    }
}
pub(super) fn fitted(source: &Mesh, layer: u8) -> Mesh {
    let mut out = source.clone();
    let normals = match source.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(n)) => n.clone(),
        _ => return out,
    };
    if let Some(VertexAttributeValues::Float32x3(p)) = out.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for (p, n) in p.iter_mut().zip(normals) {
            *p = (Vec3::from_array(*p)
                + Vec3::from_array(n).normalize_or_zero() * if layer == 0 { 0.024 } else { 0.018 })
            .to_array();
        }
    }
    out
}
pub(super) fn conceal_skin(mesh: &mut Mesh) {
    let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        return;
    };
    let Some(indices) = mesh.indices() else {
        return;
    };
    let ids: Vec<_> = indices.iter().collect();
    let visible = ids
        .chunks_exact(3)
        .filter(|t| {
            layer(
                (Vec3::from_array(p[t[0]]) + Vec3::from_array(p[t[1]]) + Vec3::from_array(p[t[2]]))
                    / 3.,
            )
            .is_none()
        })
        .flatten()
        .map(|i| *i as u32)
        .collect();
    mesh.insert_indices(Indices::U32(visible));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fitted_shell_keeps_weights_and_stays_outward_under_bone_transforms() {
        let mut source = Mesh::from(Cuboid::new(0.4, 0.4, 0.3));
        let count = source.count_vertices();
        source.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1., 0., 0., 0.]; count]);
        let shell = fitted(&source, 0);
        assert_eq!(
            shell.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
            source.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        );
        let VertexAttributeValues::Float32x3(p) =
            source.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let VertexAttributeValues::Float32x3(q) =
            shell.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!()
        };
        let VertexAttributeValues::Float32x3(n) = source.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap()
        else {
            panic!()
        };
        let matrix = Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, 0.8, -0.7, 1.2))
            .with_scale(Vec3::new(0.8, 1.3, 0.7))
            .to_matrix();
        for ((a, b), n) in p.iter().zip(q).zip(n) {
            let d = matrix.transform_point3(Vec3::from_array(*b))
                - matrix.transform_point3(Vec3::from_array(*a));
            let normal = matrix
                .inverse()
                .transpose()
                .transform_vector3(Vec3::from_array(*n));
            assert!(d.dot(normal) > 0.023);
        }
    }
    #[test]
    fn covered_torso_has_no_skin_triangles_to_poke_through() {
        let mut mesh = Mesh::from(Cuboid::new(0.4, 0.4, 0.3));
        if let Some(VertexAttributeValues::Float32x3(p)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for p in p {
                p[1] += 0.95;
            }
        }
        conceal_skin(&mut mesh);
        assert_eq!(mesh.indices().unwrap().len(), 0);
        assert!(layer(Vec3::new(0., 1.4, 0.)).is_none());
        assert!(layer(Vec3::new(0.7, 1., 0.)).is_none());
    }
}
