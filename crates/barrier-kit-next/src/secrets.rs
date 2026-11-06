//! The freedesktop Secret Service (org.freedesktop.secrets), where Linux keeps what the macOS
//! Keychain keeps for iTerm2: items with a label, attributes and a secret.

use std::collections::HashMap;

use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};
use zeroize::Zeroizing;

const SERVICE: &str = "org.freedesktop.secrets";
const SERVICE_PATH: &str = "/org/freedesktop/secrets";
const SERVICE_INTERFACE: &str = "org.freedesktop.Secret.Service";
const ITEM_INTERFACE: &str = "org.freedesktop.Secret.Item";
// Secret Service API 0.2 §“Collections”: the alias for the user's default collection.
const DEFAULT_COLLECTION: &str = "/org/freedesktop/secrets/aliases/default";
const NO_PROMPT: &str = "/";

type Secret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

#[derive(Debug)]
pub enum Error {
    // The keyring asked to be unlocked; its own prompt is showing.
    Locked,
    Bus(zbus::Error),
}

impl From<zbus::Error> for Error {
    fn from(error: zbus::Error) -> Self {
        Self::Bus(error)
    }
}

impl From<zbus::zvariant::Error> for Error {
    fn from(error: zbus::zvariant::Error) -> Self {
        Self::Bus(error.into())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub path: OwnedObjectPath,
    pub label: String,
    pub attributes: HashMap<String, String>,
}

pub struct Secrets {
    connection: Connection,
    session: OwnedObjectPath,
}

impl Secrets {
    // §“Transfer of Secrets”: the plain algorithm; the bus is local to the user.
    pub fn open() -> Result<Self, Error> {
        let connection = Connection::session()?;
        let reply = connection.call_method(
            Some(SERVICE),
            SERVICE_PATH,
            Some(SERVICE_INTERFACE),
            "OpenSession",
            &("plain", Value::from("")),
        )?;
        let (_, session): (OwnedValue, OwnedObjectPath) = reply.body().deserialize()?;
        Ok(Self {
            connection,
            session,
        })
    }

    fn call<B, R>(&self, path: &str, interface: &str, method: &str, body: &B) -> Result<R, Error>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> serde::Deserialize<'d> + zbus::zvariant::Type,
    {
        let reply =
            self.connection
                .call_method(Some(SERVICE), path, Some(interface), method, body)?;
        Ok(reply.body().deserialize()?)
    }

    // §“Prompts”: a prompt other than “/” is shown by the service, which answers later.
    fn prompted(&self, prompt: &OwnedObjectPath) -> Result<(), Error> {
        if prompt.as_str() == NO_PROMPT {
            return Ok(());
        }
        let _: () = self.call(
            prompt.as_str(),
            "org.freedesktop.Secret.Prompt",
            "Prompt",
            &"",
        )?;
        Err(Error::Locked)
    }

    fn property(&self, path: &str, name: &str) -> Result<OwnedValue, Error> {
        self.call(
            path,
            "org.freedesktop.DBus.Properties",
            "Get",
            &(ITEM_INTERFACE, name),
        )
    }

    pub fn search(&self, attributes: &HashMap<&str, &str>) -> Result<Vec<Item>, Error> {
        let (mut unlocked, locked): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) =
            self.call(SERVICE_PATH, SERVICE_INTERFACE, "SearchItems", attributes)?;
        if !locked.is_empty() {
            let (opened, prompt): (Vec<OwnedObjectPath>, OwnedObjectPath) =
                self.call(SERVICE_PATH, SERVICE_INTERFACE, "Unlock", &locked)?;
            self.prompted(&prompt)?;
            unlocked.extend(opened);
        }
        unlocked
            .into_iter()
            .map(|path| {
                let label = String::try_from(self.property(path.as_str(), "Label")?)?;
                let attributes = HashMap::<String, String>::try_from(
                    self.property(path.as_str(), "Attributes")?,
                )?;
                Ok(Item {
                    path,
                    label,
                    attributes,
                })
            })
            .collect()
    }

    pub fn secret(&self, item: &Item) -> Result<Zeroizing<Vec<u8>>, Error> {
        let (_, _, value, _): Secret = self.call(
            item.path.as_str(),
            ITEM_INTERFACE,
            "GetSecret",
            &self.session,
        )?;
        Ok(Zeroizing::new(value))
    }

    fn wrapped(&self, secret: &[u8]) -> Secret {
        (
            self.session.clone(),
            Vec::new(),
            secret.to_vec(),
            "text/plain".to_string(),
        )
    }

    pub fn create(
        &self,
        label: &str,
        attributes: &HashMap<&str, &str>,
        secret: &[u8],
    ) -> Result<(), Error> {
        let mut properties: HashMap<&str, Value> = HashMap::new();
        properties.insert("org.freedesktop.Secret.Item.Label", Value::from(label));
        properties.insert(
            "org.freedesktop.Secret.Item.Attributes",
            Value::from(attributes.clone()),
        );
        let (_, prompt): (OwnedObjectPath, OwnedObjectPath) = self.call(
            DEFAULT_COLLECTION,
            "org.freedesktop.Secret.Collection",
            "CreateItem",
            &(properties, self.wrapped(secret), true),
        )?;
        self.prompted(&prompt)
    }

    pub fn set_secret(&self, item: &Item, secret: &[u8]) -> Result<(), Error> {
        let _: () = self.call(
            item.path.as_str(),
            ITEM_INTERFACE,
            "SetSecret",
            &(self.wrapped(secret),),
        )?;
        Ok(())
    }

    pub fn delete(&self, item: &Item) -> Result<(), Error> {
        let prompt: OwnedObjectPath =
            self.call(item.path.as_str(), ITEM_INTERFACE, "Delete", &())?;
        self.prompted(&prompt)
    }
}

impl Drop for Secrets {
    fn drop(&mut self) {
        let path = ObjectPath::from(&self.session);
        let _ = self.connection.call_method(
            Some(SERVICE),
            path,
            Some("org.freedesktop.Secret.Session"),
            "Close",
            &(),
        );
    }
}
