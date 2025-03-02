#[derive(PartialEq, Eq)]
pub enum GeoCmd {
    MoveTo,
    LineTo,
    ClosePath,
    UnKnown,
}
pub struct CommandInteger(u32);
impl CommandInteger {
    pub fn id(&self) -> GeoCmd {
        match self.0 & 0x07 {
            0x01 => GeoCmd::MoveTo,
            0x02 => GeoCmd::LineTo,
            0x07 => GeoCmd::ClosePath,
            _ => GeoCmd::UnKnown,
        }
    }

    pub fn count(&self) -> u32 {
        self.0 >> 3
    }
}
pub struct ParameterInteger(u32);
impl ParameterInteger {
    pub fn value(&self) -> i32 {
        (self.0 as i32 >> 1) ^ (-(self.0 as i32 & 1))
    }
}

pub mod tile {
    use std::collections::HashMap;

    use crate::{
        mvt::{GeoCmd, ParameterInteger},
        pb::vector_tile::tile::{self},
    };
    use prost::{DecodeError, Message};

    use super::CommandInteger;

    pub struct Tile {
        pub layers: Vec<Layer>,
    }
    impl Tile {
        pub fn decode(bin: &[u8]) -> Result<Tile, DecodeError> {
            let tile = crate::pb::vector_tile::Tile::decode(bin)?;
            Ok(Self {
                layers: tile.layers.into_iter().map(|l| Layer::from(l)).collect(),
            })
        }
    }

    pub struct Layer {
        pub version: u32,
        pub name: String,
        pub features: Vec<Feature>,
    }
    impl Layer {
        fn from(layer: tile::Layer) -> Self {
            // println!("layer name:{}", layer.name);
            let extent = layer.extent() as f32;
            let keys = layer.keys;
            let values = layer.values;
            Self {
                version: layer.version,
                name: layer.name,
                features: layer
                    .features
                    .into_iter()
                    .map(|f| Feature::from(&keys, &values, extent, f))
                    .collect(),
            }
        }
    }

    pub struct Feature {
        pub id: u64,
        pub geometry: Geometry,
        pub props: HashMap<String, Value>,
    }
    impl Feature {
        fn from(
            keys: &Vec<String>,
            values: &Vec<Value>,
            extent: f32,
            feature: tile::Feature,
        ) -> Self {
            let mut props = HashMap::new();
            let mut tags = feature.tags.iter();
            while let Some(k) = tags.next() {
                let v = tags.next().unwrap();
                props.insert(keys[*k as usize].clone(), values[*v as usize].clone());
            }
            let geometry = match feature.r#type() {
                GeomType::Point => {
                    let cmd = CommandInteger(feature.geometry[0]);
                    assert!(cmd.id() == GeoCmd::MoveTo);
                    assert!(cmd.count() > 0);
                    // println!("count:{}",cmd.count());
                    let mut iter = (1..=(cmd.count() * 2))
                        .map(|i| ParameterInteger(feature.geometry[i as usize]).value());
                    let mut vec = Vec::new();
                    while let Some(i1) = iter.next() {
                        let f2 = iter.next().unwrap() as f32;
                        vec.push((i1 as f32 / extent, f2 / extent));
                    }
                    // println!("len:{}",vec.len());
                    Geometry::Point { points: vec }
                }
                // GeomType::Linestring => {
                //     let iter = feature.geometry.iter();

                // }
                _ => Geometry::UnKnown,
            };
            Self {
                id: feature.id(),
                geometry,
                props,
            }
        }
    }

    pub type GeomType = tile::GeomType;
    pub type Value = tile::Value;
    pub enum Geometry {
        UnKnown,
        Point { points: Vec<(f32, f32)> },
    }
}
