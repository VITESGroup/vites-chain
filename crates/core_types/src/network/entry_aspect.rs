use crate::{
    chain_header::ChainHeader, entry::Entry, error::HolochainError, link::link_data::LinkData,
};
use holochain_json_api::{error::JsonError, json::JsonString};
use holochain_persistence_api::cas::content::{Address, AddressableContent, Content};
use std::{
    convert::{Into, TryFrom},
    fmt,
    hash::{Hash, Hasher},
};

impl AddressableContent for EntryAspect {
    fn content(&self) -> Content {
        self.to_owned().into()
    }

    fn try_from_content(content: &Content) -> Result<Self, JsonError> {
        Self::try_from(content.to_owned())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, DefaultJson, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum EntryAspect {
    // Basic case: entry content is communicated
    // with its header.
    // Content alone never makes sense
    // (receiveing node needs header and especially
    // source to run validation)
    Content(Entry, ChainHeader),

    // Communicating only the header makes sense if an
    // entry was deleted but we need to remember that
    // there was an entry that got deleted (sacrileged)
    Header(ChainHeader),

    // This is the meta item for adding a link.
    // The ChainHeader is needed for validation of
    // this LinkAdd statement/entry.
    // (Instead of type `LinkData` we could also have
    // an `Entry` that would always be expected to the be
    // `Entry::LinkAdd` specialisation, but in order to make
    // impossible states impossible we choose `LinkData`.
    // Putting that `LinkData` in an `Entry::LinkAdd` should
    // result in the exact same entry the `ChainHeader` is
    // a header for)
    LinkAdd(LinkData, ChainHeader),

    // Same as LinkAdd but for removal of links
    // TODO: can this tuple be combined with EntryType::LinkRemove's data, which is the same?
    LinkRemove((LinkData, Vec<Address>), ChainHeader),

    // TODO this looks wrong to me.  I don't think we actually want to
    // send the updated Entry as part of the meta item.  That would mean the
    // new entry is getting stored two places on the dht.  I think this
    // should look the same same as Deletion
    // AND, we don't actually need to even have the Address as part of the
    // Variant because the correct value is already in the Chain Header
    // as the link_update_delete attribute
    // Meta item for updating an entry.
    // The given Entry is the new version and ChainHeader
    // the header of the new version.
    // The header's CRUD link must reference the base address
    // of the EntryData this is in.
    //  Update(Entry, ChainHeader),
    Update(Entry, ChainHeader),

    // Meta item for removing an entry.
    // Address is the address of the deleted entry.
    // ChainHeader is the header of that deletion entry that
    // could be assembled by putting the address in an
    // `Entry::Deletion(address)`.
    // Deletion(Address, ChainHeader),
    Deletion(ChainHeader),
}

impl EntryAspect {
    pub fn type_hint(&self) -> String {
        match self {
            EntryAspect::Content(_, _) => String::from("content"),
            EntryAspect::Header(_) => String::from("header"),
            EntryAspect::LinkAdd(_, _) => String::from("link_add"),
            EntryAspect::LinkRemove(_, _) => String::from("link_remove"),
            EntryAspect::Update(_, _) => String::from("update"),
            EntryAspect::Deletion(_) => String::from("deletion"),
        }
    }
    pub fn header(&self) -> &ChainHeader {
        match self {
            EntryAspect::Content(_, header) => header,
            EntryAspect::Header(header) => header,
            EntryAspect::LinkAdd(_, header) => header,
            EntryAspect::LinkRemove(_, header) => header,
            EntryAspect::Update(_, header) => header,
            EntryAspect::Deletion(header) => header,
        }
    }
    /// NB: this is the inverse function of entry_to_meta_aspect,
    /// so it is very important that they agree!
    /// NOTE: the ContentAspect address is always the entry address and this
    /// is not used by entry_to_meta_aspect
    pub fn entry_address(&self) -> Result<Address, HolochainError> {
        Ok(match self {
            EntryAspect::Content(_, header) => header.entry_address().clone(),
            EntryAspect::LinkAdd(link_data, _) => link_data.link.base().clone(),
            EntryAspect::LinkRemove((link_data, _), _) => link_data.link.base().clone(),
            EntryAspect::Update(_, header) | EntryAspect::Deletion(header) => {
                header.link_update_delete().ok_or_else(|| {
                    HolochainError::ErrorGeneric(format!(
                        "no link_update_delete on Update/Deletion entry header. Header: {:?}",
                        header
                    ))
                })?
            }
            // EntryAspect::Header is currently unused,
            // but this is what it will be when we do use it
            EntryAspect::Header(header) => header.address(),
        })
    }
}

fn format_header(header: &ChainHeader) -> String {
    format!(
        "Header[type: {}, crud_link: {:?}]",
        header.entry_type(),
        header.link_update_delete()
    )
}
impl fmt::Debug for EntryAspect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EntryAspect::Content(entry, header) => write!(
                f,
                "EntryAspect::Content({}, {})",
                entry.address(),
                format_header(header)
            ),
            EntryAspect::Header(header) => {
                write!(f, "EntryAspect::Header({})", format_header(header))
            }
            EntryAspect::LinkAdd(link_data, header) => write!(
                f,
                "EntryAspect::LinkAdd({} -> {} [tag: {}, type: {}], {})",
                link_data.link.base(),
                link_data.link.target(),
                link_data.link.tag(),
                link_data.link.link_type(),
                format_header(header)
            ),
            EntryAspect::LinkRemove((link_data, _), header) => write!(
                f,
                "EntryAspect::LinkRemove({} -> {} [tag: {}, type: {}], top_chain_header:{}, remove_header: {})",
                link_data.link.base(),
                link_data.link.target(),
                link_data.link.tag(),
                link_data.link.link_type(),
                format_header(&link_data.top_chain_header),
                format_header(header)
            ),
            EntryAspect::Update(entry, header) => write!(
                f,
                "EntryAspect::Update({}, {})",
                entry.address(),
                format_header(header)
            ),
            EntryAspect::Deletion(header) => {
                write!(f, "EntryAspect::Deletion({})", format_header(header))
            }
        }
    }
}

#[allow(clippy::derive_hash_xor_eq)]
// This clippy lint stresses the point that impls of Hash and PartialEq have to agree,
// that is ensure that: k1 == k2 ??? hash(k1) == hash(k2).
// In this custom Hash impl I'm just taking the entry address into account.
// The derived PartialEq takes all fields into account. If all fields are the same, so must
// the entry addresses which is part of all. QED.
impl Hash for EntryAspect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.header().hash(state);
        self.type_hint().hash(state);
    }
}
