use crate::{core::Logs, Docker, Image};
use std::env::var;

/// Represents a running docker container.
///
/// Containers have a [`custom destructor`][drop_impl] that removes them as soon as they go out of scope:
///
/// ```rust
/// use testcontainers::*;
/// #[test]
/// fn a_test() {
///     let docker = clients::Cli::default();
///
///     {
///         let container = docker.run(MyImage::default());
///
///         // Docker container is stopped/removed at the end of this scope.
///     }
/// }
///
/// ```
///
/// [drop_impl]: struct.Container.html#impl-Drop
#[derive(Debug)]
pub struct Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    id: String,
    docker_client: &'d D,
    image: I,
}

impl<'d, D, I> Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    /// Constructs a new container given an id, a docker client and the image.
    ///
    /// This function will block the current thread (if [`wait_until_ready`] is implemented correctly) until the container is actually ready to be used.
    ///
    /// [`wait_until_ready`]: trait.Image.html#tymethod.wait_until_ready
    pub fn new(id: String, docker_client: &'d D, image: I) -> Self {
        let container = Container {
            id,
            docker_client,
            image,
        };

        container.block_until_ready();

        container
    }

    /// Returns the id of this container.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Gives access to the log streams of this container.
    pub fn logs(&self) -> Logs {
        self.docker_client.logs(&self.id)
    }

    /// Returns the mapped host port for an internal port of this docker container.
    ///
    /// This method does **not** magically expose the given port, it simply performs a mapping on
    /// the already exposed ports. If a docker image does not expose a port, this method will not
    /// be able to resolve it.
    pub fn get_host_port(&self, internal_port: u16) -> Option<u16> {
        let resolved_port = self
            .docker_client
            .ports(&self.id)
            .map_to_host_port(internal_port);

        match resolved_port {
            Some(port) => {
                log::debug!(
                    "Resolved port {} to {} for container {}",
                    internal_port,
                    port,
                    self.id
                );
            }
            None => {
                log::warn!(
                    "Unable to resolve port {} for container {}",
                    internal_port,
                    self.id
                );
            }
        }

        resolved_port
    }

    /// Returns a reference to the [`Image`] of this container.
    ///
    /// Access to this is useful if the [`arguments`] of the [`Image`] change how to connect to the
    /// Access to this is useful to retrieve [`Image`] specific information such as authentication details or other relevant information which have been passed as [`arguments`]
    ///
    /// [`Image`]: trait.Image.html
    /// [`arguments`]: trait.Image.html#associatedtype.Args
    pub fn image(&self) -> &I {
        &self.image
    }

    fn block_until_ready(&self) {
        log::debug!("Waiting for container {} to be ready", self.id);

        self.image.wait_until_ready(self);

        log::debug!("Container {} is now ready!", self.id);
    }

    pub fn stop(&self) {
        log::debug!("Stopping docker container {}", self.id);

        self.docker_client.stop(&self.id)
    }

    pub fn start(&self) {
        self.docker_client.start(&self.id);
    }

    pub fn rm(&self) {
        log::debug!("Deleting docker container {}", self.id);

        self.docker_client.rm(&self.id)
    }
}

/// The destructor implementation for a Container.
///
/// As soon as the container goes out of scope, the destructor will either only stop or delete the docker container.
/// This behaviour can be controlled through the `KEEP_CONTAINERS` environment variable. Setting it to `true` will only stop containers instead of removing them. Any other or no value will remove the container.
impl<'d, D, I> Drop for Container<'d, D, I>
where
    D: Docker,
    I: Image,
{
    fn drop(&mut self) {
        let keep_container = var("KEEP_CONTAINERS")
            .ok()
            .and_then(|var| var.parse().ok())
            .unwrap_or(false);

        match keep_container {
            true => self.stop(),
            false => self.rm(),
        }
    }
}
