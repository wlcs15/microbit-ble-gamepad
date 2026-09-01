use super::advertiser::{Advertiser, AdvertiserBuilder};
use super::{ble_task, mpsl_task, BleResources};
use super::{hid::*, BleServer};
use super::{stick::*, BleController};
use defmt::{info, warn};
use embassy_executor::Spawner;
use microbit_bsp::ble::{MultiprotocolServiceLayer, SoftdeviceError};
use static_cell::StaticCell;
use trouble_host::prelude::*;

/// Allow a central to decide which player this controller belongs to
#[gatt_service(uuid = "8f701cf1-b1df-42a1-bb5f-6a1028c793b0")]
pub struct Player {
    #[characteristic(uuid = "e3d1afe4-b414-44e3-be54-0ea26c394eba", read, write, notify)]
    index: u8,
}

#[gatt_server]
pub struct Server {
    pub hid: ButtonService,
    pub stick: StickService,
    pub player: Player,
}

impl BleServer {
    pub fn start_gatt(
        name: &'static str,
        spawner: Spawner,
        controller: BleController,
        mpsl: &'static MultiprotocolServiceLayer<'static>,
    ) -> Result<(&'static Self, Advertiser<'static, BleController>), BleHostError<SoftdeviceError>>
    {
        spawner.must_spawn(mpsl_task(mpsl));

        let address = Address::random([0x42, 0x5A, 0xE3, 0x1E, 0x83, 0xE7]);
        info!("Our address = {:?}", address);

        let resources = {
            static RESOURCES: StaticCell<BleResources> = StaticCell::new();
            RESOURCES.init(BleResources::new())
        };
        let stack = {
            static STACK: StaticCell<Stack<'static, BleController, DefaultPacketPool>> =
                StaticCell::new();
            STACK.init(trouble_host::new(controller, resources).set_random_address(address))
        };
        let Host {
            peripheral, runner, ..
        } = stack.build();
        let server = {
            static SERVER: StaticCell<BleServer> = StaticCell::new();
            SERVER.init(
                Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
                    name,
                    appearance: &appearance::human_interface_device::GAMEPAD,
                }))
                .expect("Error creating Gatt Server"),
            )
        };
        info!("Starting Gatt Server");
        spawner.must_spawn(ble_task(runner));
        let advertiser = AdvertiserBuilder::new(name, peripheral).build()?;
        Ok((server, advertiser))
    }
}

/// A BLE GATT server
pub async fn gatt_server_task(
    server: &BleServer,
    conn: &GattConnection<'_, '_, DefaultPacketPool>,
) {
    loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => {
                info!("[gatt] Disconnected: {:?}", reason);
                break;
            }
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(read) => {
                        if read.handle() == server.player.index.handle {
                            let value = server.get(&server.player.index);
                            info!(
                                "[gatt] Read Event to Player Index Characteristic: {:?}",
                                value
                            );
                        }
                    }
                    GattEvent::Write(write) => {
                        if write.handle() == server.player.index.handle {
                            let value = server.get(&server.player.index);
                            info!(
                                "[gatt] Write Event to Player Index Characteristic: {:?}",
                                value
                            );
                        }
                    }
                    _ => {}
                }
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                }
            }
            _ => {}
        }
    }
    info!("Gatt server task finished");
}
