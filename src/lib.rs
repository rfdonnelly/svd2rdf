use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use svd_rs as svd;

#[derive(Serialize, Deserialize, Debug)]
pub struct Rdf {
    schema: Schema,
    design: Design,
    root: Root,
    elements: IndexMap<String, Element>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Schema {
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Design {
    name: String,
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Root {
    children: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Element {
    #[serde(flatten)]
    typ: ElementType,
    id: String,
    name: String,
    addr: String,
    offset: String,
    doc: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ElementType {
    #[serde(rename = "blk")]
    Blk { children: Vec<String> },
    #[serde(rename = "reg")]
    Reg { fields: Vec<Field> },
    #[serde(rename = "mem")]
    Mem,
}

#[derive(Serialize, Deserialize, Debug)]
struct Field {
    name: String,
    lsb: u32,
    nbits: u32,
    access: String,
    reset: String,
    doc: String,
}

impl From<svd::Device> for Rdf {
    fn from(device: svd::Device) -> Self {
        let mut elements = IndexMap::new();

        let children = device
            .peripherals
            .iter()
            .map(|peripheral| match peripheral {
                svd::MaybeArray::Single(peripheral) => peripheral.name.to_lowercase(),
                svd::MaybeArray::Array(_peripheral, _dim) => unimplemented!(),
            })
            .collect();

        for peripheral in device.peripherals {
            match peripheral {
                svd::MaybeArray::Single(peripheral) => visit_periperal(peripheral, &mut elements),
                svd::MaybeArray::Array(_peripheral, _dim) => unimplemented!(),
            }
        }

        Self {
            schema: Schema {
                version: "v0.1".into(),
            },
            design: Design {
                name: device.name,
                version: device.version,
            },
            root: Root { children },
            elements,
        }
    }
}

fn visit_periperal(peripheral: svd::PeripheralInfo, elements: &mut IndexMap<String, Element>) {
    let name = peripheral.name.to_lowercase();
    let path = name.clone();

    let mut children = Vec::new();
    peripheral_child_ids(&path, &peripheral, &mut children);

    let element = Element {
        typ: ElementType::Blk { children },
        id: name.clone(),
        name,
        addr: format!("0x{:x}", peripheral.base_address),
        offset: format!("0x{:x}", peripheral.base_address),
        doc: peripheral.description.unwrap_or_else(|| "undefined".to_string()),
    };

    elements.insert(element.id.clone(), element);

    match peripheral.registers {
        Some(register_clusters) => {
            for register_cluster in register_clusters {
                match register_cluster {
                    svd::RegisterCluster::Register(register) => match register {
                        svd::MaybeArray::Single(register) => {
                            visit_register(&path, register, elements)
                        }
                        svd::MaybeArray::Array(register, dim) => {
                            let name = register.name.to_lowercase();
                            for i in 0..dim.dim {
                                let child_name = name.replace("%s", &i.to_string());
                                let child_id = format!("{}.{}", path, child_name);

                                let mut fields = Vec::new();
                                collect_fields(&register, &mut fields);

                                let element = Element {
                                    typ: ElementType::Reg { fields },
                                    id: child_id,
                                    name: child_name,
                                    addr: format!("0x{:x}", register.address_offset + i * dim.dim_increment),
                                    offset: format!("0x{:x}", register.address_offset + i * dim.dim_increment),
                                    doc: register.description.clone().unwrap_or_else(|| "undefined".to_string()),
                                };

                                elements.insert(element.id.clone(), element);
                            }
                        }
                    }
                    svd::RegisterCluster::Cluster(_cluster) => unimplemented!(),
                }
            }
        }
        None => {}
    }
}

fn peripheral_child_ids(path: &str, peripheral: &svd::PeripheralInfo, child_ids: &mut Vec<String>) {
    match &peripheral.registers {
        Some(registers) => registers
            .iter()
            .for_each(|register_cluster| match register_cluster {
                svd::RegisterCluster::Register(register) => match register {
                    svd::MaybeArray::Single(register) => {
                        let name = register.name.to_lowercase();
                        child_ids.push(format!("{}.{}", path, name))
                    }
                    svd::MaybeArray::Array(register, dim) => {
                        let name = register.name.to_lowercase();
                        for i in 0..dim.dim {
                            let child_name = name.replace("%s", &i.to_string());
                            let child_id = format!("{}.{}", path, child_name);
                            child_ids.push(child_id);
                        }
                    }
                },
                svd::RegisterCluster::Cluster(_cluster) => unimplemented!(),
            }),
        None => {}
    }
}

fn collect_fields(register: &svd::RegisterInfo, fields: &mut Vec<Field>) {
    match &register.fields {
        Some(svd_fields) => {
            for field in svd_fields {
                match field {
                    svd::MaybeArray::Single(field) => {
                        let name = field.name.to_lowercase();

                        let access = match field.access {
                            Some(access) => access.as_str().to_string(),
                            None => "undefined".to_string(),
                        };

                        let field = Field {
                            name,
                            lsb: field.bit_range.offset,
                            nbits: field.bit_range.width,
                            access,
                            reset: "0x0".into(),
                            doc: field.description.clone().unwrap_or_else(|| "undefined".to_string()),
                        };

                        fields.push(field);
                    }
                    svd::MaybeArray::Array(field, dim) => {
                        todo!();
                    }
                }
            }
        }
        None => {},
    }
}

fn visit_register(path: &str, register: svd::RegisterInfo, elements: &mut IndexMap<String, Element>) {
    let mut fields = Vec::new();
    collect_fields(&register, &mut fields);

    let name = register.name.to_lowercase();
    let element = Element {
        typ: ElementType::Reg { fields },
        id: format!("{}.{}", path, name),
        name,
        addr: format!("0x{:x}", register.address_offset),
        offset: format!("0x{:x}", register.address_offset),
        doc: register.description.unwrap_or_else(|| "undefined".to_string()),
    };

    elements.insert(element.id.clone(), element);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn example() {
        let svd = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/input/example.svd"));
        let device = svd_parser::parse(svd).unwrap();
        let rdf = Rdf::from(device);

        insta::assert_yaml_snapshot!(rdf);
    }
}
