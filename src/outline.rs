// Adapted from: https://github.com/komadori/bevy_mod_outline

use bevy::{
    math::DVec3,
    prelude::*,
    render::{
        mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues},
        render_resource::VertexFormat,
    },
    utils::{FloatOrd, HashMap},
};

/// The direction to extrude the vertex when rendering the outline.
pub const ATTRIBUTE_OUTLINE_NORMAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Outline_Normal", 1585570526, VertexFormat::Float32x3);

pub fn generate_outline_mesh(mesh: &Mesh, thickness: f32) -> Result<Mesh, GenerateOutlineError> {
    let mut outline_mesh = mesh.clone();

    smooth_normals(&mut outline_mesh)?;
    move_vertices_along_normals(&mut outline_mesh, thickness)?;
    Ok(outline_mesh)
}

pub fn smooth_normals(mesh: &mut Mesh) -> Result<(), GenerateOutlineError> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return Err(GenerateOutlineError::UnsupportedPrimitiveTopology(
            mesh.primitive_topology(),
        ));
    }
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).ok_or(
        GenerateOutlineError::MissingVertexAttribute(Mesh::ATTRIBUTE_POSITION.name),
    )? {
        VertexAttributeValues::Float32x3(p) => Ok(p),
        v => Err(GenerateOutlineError::InvalidVertexAttributeFormat(
            Mesh::ATTRIBUTE_POSITION.name,
            VertexFormat::Float32x3,
            v.into(),
        )),
    }?;
    let normals = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(p)) => Some(p),
        _ => None,
    };

    let mut map = HashMap::<[FloatOrd; 3], DVec3>::with_capacity(positions.len());

    // iteration the complicated way... don't know  a better way to do this without heap allocating
    enum IndicesIter<'a> {
        U16(std::slice::Iter<'a, u16>),
        U32(std::slice::Iter<'a, u32>),
        None(std::ops::Range<usize>),
    }
    let mut it = match mesh.indices() {
        Some(Indices::U16(it)) => IndicesIter::U16(it.iter()),
        Some(Indices::U32(it)) => IndicesIter::U32(it.iter()),
        None => IndicesIter::None(0..positions.len()),
    };
    let mut it = std::iter::from_fn(move || match &mut it {
        IndicesIter::U16(it) => it.next().map(|i| *i as usize),
        IndicesIter::U32(it) => it.next().map(|i| *i as usize),
        IndicesIter::None(it) => it.next(),
    });

    while let (Some(i0), Some(i1), Some(i2)) = (it.next(), it.next(), it.next()) {
        for (j0, j1, j2) in [(i0, i1, i2), (i1, i2, i0), (i2, i0, i1)] {
            const SCALE: f64 = 1e8;
            let p0 = Vec3::from(positions[j0]);
            let p1 = Vec3::from(positions[j1]);
            let p2 = Vec3::from(positions[j2]);
            let v1 = DVec3::from(p1 - p0) * SCALE;
            let v2 = DVec3::from(p2 - p0) * SCALE;
            let angle = (v1).angle_between(v2);
            let n = map
                .entry([
                    FloatOrd(p0.x as f32),
                    FloatOrd(p0.y as f32),
                    FloatOrd(p0.z as f32),
                ])
                .or_default();
            *n += angle * v1.cross(v2).normalize_or_zero();

            // if let Some(ns) = normals {
            //     // Use vertex normal
            //     DVec3::from(Vec3::from(ns[j0]))
            // } else {
            //     // Calculate face normal
            //     (p1 - p0).cross(p2 - p0).normalize_or_zero()
            // };
        }
    }

    let mut outlines = Vec::with_capacity(positions.len());
    for p in positions.iter() {
        let key = [FloatOrd(p[0]), FloatOrd(p[1]), FloatOrd(p[2])];
        let v = map
            .get(&key)
            .copied()
            .unwrap_or(DVec3::ZERO)
            .normalize_or_zero();
        outlines.push([v.x as f32, v.y as f32, v.z as f32]);
    }

    mesh.insert_attribute(
        ATTRIBUTE_OUTLINE_NORMAL,
        VertexAttributeValues::Float32x3(outlines),
    );
    Ok(())
}

/// Moves the vertices of the mesh along their normals by distance.
pub fn move_vertices_along_normals(
    mesh: &mut Mesh,
    distance: f32,
) -> Result<(), GenerateOutlineError> {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).ok_or(
        GenerateOutlineError::MissingVertexAttribute(Mesh::ATTRIBUTE_POSITION.name),
    )? {
        VertexAttributeValues::Float32x3(p) => Ok(p),
        v => Err(GenerateOutlineError::InvalidVertexAttributeFormat(
            Mesh::ATTRIBUTE_POSITION.name,
            VertexFormat::Float32x3,
            v.into(),
        )),
    }?;
    let normals = match mesh.attribute(ATTRIBUTE_OUTLINE_NORMAL).ok_or(
        GenerateOutlineError::MissingVertexAttribute(ATTRIBUTE_OUTLINE_NORMAL.name),
    )? {
        VertexAttributeValues::Float32x3(p) => Ok(p),
        v => Err(GenerateOutlineError::InvalidVertexAttributeFormat(
            ATTRIBUTE_OUTLINE_NORMAL.name,
            VertexFormat::Float32x3,
            v.into(),
        )),
    }?;

    let mut new_positions = Vec::with_capacity(positions.len());
    for (p, n) in positions.iter().zip(normals.iter()) {
        new_positions.push([
            p[0] + n[0] * distance,
            p[1] + n[1] * distance,
            p[2] + n[2] * distance,
        ]);
    }
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(new_positions),
    );
    Ok(())
}

/// Failed to generate outline normals for the mesh.
#[derive(thiserror::Error, Debug)]
pub enum GenerateOutlineError {
    #[error("unsupported primitive topology '{0:?}'")]
    UnsupportedPrimitiveTopology(PrimitiveTopology),
    #[error("missing vertex attributes '{0}'")]
    MissingVertexAttribute(&'static str),
    #[error("the '{0}' vertex attribute should have {1:?} format, but had {2:?} format")]
    InvalidVertexAttributeFormat(&'static str, VertexFormat, VertexFormat),
}
