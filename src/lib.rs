use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use svd_rs as svd;

#[derive(Serialize, Deserialize, Debug)]
pub struct Rdf {
    schema: Schema,
    root: Root,
    elements: IndexMap<String, Element>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Schema {
    version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Root {
    name: String,
    display_name: String,
    version: String,
    children: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Element {
    #[serde(flatten)]
    typ: ElementType,
    id: String,
    name: String,
    addr: u32,
    offset: u32,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Field {
    name: String,
    lsb: u32,
    nbits: u32,
    access: String,
    reset: u32,
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
                version: "v0.2".into(),
            },
            root: Root {
                name: device.name.clone(),
                display_name: device.name,
                version: device.version,
                children,
            },
            elements,
        }
    }
}

fn visit_periperal(peripheral: svd::PeripheralInfo, elements: &mut IndexMap<String, Element>) {
    let name = peripheral.name.to_lowercase();
    let path = name.clone();
    let base_address: u32 = peripheral.base_address.try_into().unwrap();

    let children = peripheral_child_ids(&path, &peripheral);

    let element = Element {
        typ: ElementType::Blk { children },
        id: path.clone(),
        name,
        addr: base_address,
        offset: base_address,
        doc: peripheral.description.unwrap_or_else(String::new),
    };

    elements.insert(element.id.clone(), element);

    if let Some(register_clusters) = peripheral.registers {
        visit_register_clusters(&path, base_address, &register_clusters, elements);
    }
}

fn visit_register_clusters(
    path: &str,
    base_address: u32,
    register_clusters: &[svd::RegisterCluster],
    elements: &mut IndexMap<String, Element>,
) {
    for register_cluster in register_clusters {
        match register_cluster {
            svd::RegisterCluster::Register(register) => match register {
                svd::MaybeArray::Single(register) => {
                    visit_register(path, base_address, register, 1, None, elements)
                }
                svd::MaybeArray::Array(register, dim) => visit_register(
                    path,
                    base_address,
                    register,
                    dim.dim,
                    Some(dim.dim_increment),
                    elements,
                ),
            },
            svd::RegisterCluster::Cluster(cluster) => match cluster {
                svd::MaybeArray::Single(cluster) => {
                    visit_cluster(path, base_address, cluster, 1, None, elements)
                }
                svd::MaybeArray::Array(cluster, dim) => visit_cluster(
                    path,
                    base_address,
                    cluster,
                    dim.dim,
                    Some(dim.dim_increment),
                    elements,
                ),
            },
        }
    }
}

fn visit_cluster(
    path: &str,
    base_address: u32,
    cluster: &svd::ClusterInfo,
    instance_count: u32,
    address_increment: Option<u32>,
    elements: &mut IndexMap<String, Element>,
) {
    let name = cluster.name.to_lowercase();

    for i in 0..instance_count {
        let name = instance_id(&name, i);
        let path = format!("{}.{}", path, name);
        let children = cluster_child_ids(&path, cluster);
        let instance_offset = cluster.address_offset + i * address_increment.unwrap_or_default();
        let element = Element {
            typ: ElementType::Blk {
                children: children.clone(),
            },
            id: path.clone(),
            name,
            addr: base_address + instance_offset,
            offset: instance_offset,
            doc: cluster.description.clone().unwrap_or_else(String::new),
        };

        elements.insert(element.id.clone(), element);

        let base_address = base_address + cluster.address_offset;
        visit_register_clusters(&path, base_address, &cluster.children, elements);
    }
}

fn instance_id(name: &str, i: u32) -> String {
    name.replace("[%s]", &i.to_string())
}

fn register_ids(path: &str, register: &svd::RegisterInfo, instances: u32, ids: &mut Vec<String>) {
    let name = register.name.to_lowercase();

    for i in 0..instances {
        let name = instance_id(&name, i);
        let id = format!("{}.{}", path, name);
        ids.push(id);
    }
}

fn cluster_ids(path: &str, cluster: &svd::ClusterInfo, instances: u32, ids: &mut Vec<String>) {
    let name = cluster.name.to_lowercase();

    for i in 0..instances {
        let name = instance_id(&name, i);
        let id = format!("{}.{}", path, name);
        ids.push(id);
    }
}

fn peripheral_child_ids(path: &str, peripheral: &svd::PeripheralInfo) -> Vec<String> {
    let mut child_ids = Vec::new();

    if let Some(register_clusters) = &peripheral.registers {
        register_clusters
            .iter()
            .for_each(|register_cluster| match register_cluster {
                svd::RegisterCluster::Register(register) => match register {
                    svd::MaybeArray::Single(register) => {
                        register_ids(path, register, 1, &mut child_ids);
                    }
                    svd::MaybeArray::Array(register, dim) => {
                        register_ids(path, register, dim.dim, &mut child_ids);
                    }
                },
                svd::RegisterCluster::Cluster(cluster) => match cluster {
                    svd::MaybeArray::Single(cluster) => {
                        cluster_ids(path, cluster, 1, &mut child_ids);
                    }
                    svd::MaybeArray::Array(cluster, dim) => {
                        cluster_ids(path, cluster, dim.dim, &mut child_ids);
                    }
                },
            })
    }

    child_ids
}

fn cluster_child_ids(path: &str, cluster: &svd::ClusterInfo) -> Vec<String> {
    let mut child_ids = Vec::new();

    cluster
        .children
        .iter()
        .for_each(|register_cluster| match register_cluster {
            svd::RegisterCluster::Register(register) => match register {
                svd::MaybeArray::Single(register) => {
                    register_ids(path, register, 1, &mut child_ids);
                }
                svd::MaybeArray::Array(register, dim) => {
                    register_ids(path, register, dim.dim, &mut child_ids);
                }
            },
            svd::RegisterCluster::Cluster(cluster) => match cluster {
                svd::MaybeArray::Single(cluster) => {
                    cluster_ids(path, cluster, 1, &mut child_ids);
                }
                svd::MaybeArray::Array(cluster, dim) => {
                    cluster_ids(path, cluster, dim.dim, &mut child_ids);
                }
            },
        });

    child_ids
}

fn collect_fields(register: &svd::RegisterInfo) -> Vec<Field> {
    let mut fields = Vec::new();

    match &register.fields {
        Some(svd_fields) => {
            for field in svd_fields {
                match field {
                    svd::MaybeArray::Single(field) => {
                        let name = field.name.to_lowercase();

                        let access = match field.access {
                            Some(access) => access.as_str().to_string(),
                            None => String::new(),
                        };

                        let field = Field {
                            name,
                            lsb: field.bit_range.offset,
                            nbits: field.bit_range.width,
                            access,
                            // FIXME: svd reset value stored in the register, need to extract for
                            // field
                            reset: 0,
                            doc: field.description.clone().unwrap_or_else(String::new),
                        };

                        fields.push(field);
                    }
                    svd::MaybeArray::Array(_field, _dim) => {
                        todo!();
                    }
                }
            }
        }
        None => {}
    }

    fields.sort_by_key(|field| field.lsb);

    let mut holes: Vec<(u32, u32)> = fields
        .windows(2)
        .filter_map(|slice| {
            let lsb = slice[0].lsb + slice[0].nbits;
            let msb = slice[1].lsb - 1;
            if msb < lsb {
                None
            } else {
                Some((lsb, msb))
            }
        })
        .collect();

    if let Some(first) = fields.first() {
        if first.lsb != 0 {
            let (lsb, msb) = (0, first.lsb - 1);
            holes.push((lsb, msb));
        }
    }

    if let Some(last) = fields.last() {
        let msb = last.lsb + last.nbits - 1;
        if msb < 31 {
            let (lsb, msb) = (msb + 1, 31);
            holes.push((lsb, msb));
        }
    }

    let mut rsvd_fields = holes
        .into_iter()
        .enumerate()
        .map(|(i, (lsb, msb))| Field {
            name: format!("rsvd{i}"),
            lsb,
            nbits: msb - lsb + 1,
            access: "rsvd".to_string(),
            reset: 0,
            doc: "Reserved".to_string(),
        })
        .collect();

    fields.append(&mut rsvd_fields);

    fields.sort_by_key(|field| field.lsb);
    fields.reverse();

    if fields.is_empty() {
        let inferred_field = Field {
            name: "inferred".to_string(),
            lsb: 0,
            nbits: 32,
            access: "inferred".to_string(),
            reset: 0,
            doc: "inferred".to_string(),
        };
        fields.push(inferred_field);
    }

    fields
}

fn visit_register(
    path: &str,
    base_address: u32,
    register: &svd::RegisterInfo,
    instance_count: u32,
    address_increment: Option<u32>,
    elements: &mut IndexMap<String, Element>,
) {
    let fields = collect_fields(register);

    let name = register.name.to_lowercase();
    for i in 0..instance_count {
        let name = instance_id(&name, i);
        let id = format!("{}.{}", path, name);
        let instance_offset = register.address_offset + i * address_increment.unwrap_or_default();

        let element = Element {
            typ: ElementType::Reg {
                fields: fields.clone(),
            },
            id,
            name,
            addr: base_address + instance_offset,
            offset: instance_offset,
            doc: register.description.clone().unwrap_or_else(String::new),
        };

        elements.insert(element.id.clone(), element);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn example() {
        let svd = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/input/example.svd"
        ));
        let device = svd_parser::parse(svd).unwrap();
        let rdf = Rdf::from(device);

        insta::assert_yaml_snapshot!(rdf);
    }
}
