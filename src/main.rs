use envconfig::Envconfig;
use lapin::Error;
use stable_eyre::eyre::{self, eyre, Result};
use std::cmp;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::runtime::Runtime;

use tokio_amqp::*;

#[macro_use]
extern crate log;

const RECV_BUF_SIZE: usize = 65535;

#[derive(Envconfig, Clone)]
pub struct Settings {
    #[envconfig(from = "U2A_UDP_BIND_ADDR")]
    pub udp_bind_addr: String,
    #[envconfig(from = "U2A_AMQP_URI")]
    pub amqp_uri: String,
    #[envconfig(from = "U2A_AMQP_EXCHANGE", default = "")]
    pub amqp_exchange: String,
    #[envconfig(from = "U2A_AMQP_ROUTING_KEY", default = "")]
    pub amqp_routing_key: String,
    #[envconfig(from = "U2A_HTTP_PROBE_PORT", default = "8080")]
    pub http_probe_port: u16,
    #[envconfig(from = "U2A_RECONNECT_DELAY_LIMIT_MS", default = "60000")]
    pub reconnect_delay_limit_ms: u64,
    #[envconfig(from = "U2A_NO_RECONNECT", default = "false")]
    pub no_reconnect: bool,
    #[envconfig(from = "U2A_DEBUG", default = "false")]
    pub debug: bool,
}

async fn tokio_main(runtime: Arc<Runtime>) -> Result<()> {
    let settings = Settings::init_from_env()?;
    setup_logging(settings.debug)?;

    let probe_state = Arc::new(AtomicBool::new(false));
    if settings.http_probe_port != 0 {
        liveness_probe_server(probe_state.clone(), &settings).await;
    }

    let mut retries = 0;
    loop {
        if let Err(why) = run(runtime.clone(), probe_state.clone(), &settings).await {
            if settings.no_reconnect {
                break;
            }

            info!("setting not ready status");
            probe_state.store(false, Ordering::Relaxed);
            error!("retrying after error: {}", why);
            tokio::time::sleep(std::time::Duration::from_millis(cmp::min(
                settings.reconnect_delay_limit_ms,
                (2 as u64).pow(retries) * 100,
            )))
            .await;
            retries += 1;
        }
    }
    Ok(())
}

async fn run(
    runtime: Arc<Runtime>,
    probe_state: Arc<AtomicBool>,
    settings: &Settings,
) -> Result<()> {
    let udp_socket = tokio::net::UdpSocket::bind(&settings.udp_bind_addr)
        .await
        .unwrap_or_else(|_| panic!("unable to bind to udp socket `{}`", settings.udp_bind_addr));
    info!("bound to udp socket `{}`", settings.udp_bind_addr);

    let (_, amqp_channel) = amqp_connect(runtime, &settings)
        .await
        .map_err(|err| match err {
            Error::IOError(why) => eyre!(why),
            _ => panic!("unable to connect to AMQP server: {}", err),
        })?;

    let mut buf = [0; RECV_BUF_SIZE];

    info!("setting ready status");
    probe_state.store(true, Ordering::Relaxed);

    loop {
        let (len, addr) = udp_socket.recv_from(&mut buf).await?;
        debug!("received {} bytes from `{}`", len, addr);
        amqp_channel
            .basic_publish(
                &settings.amqp_exchange,
                &settings.amqp_routing_key,
                lapin::options::BasicPublishOptions::default(),
                buf[..len].to_vec(),
                lapin::BasicProperties::default(),
            )
            .await?;
    }
}

async fn amqp_connect(
    runtime: Arc<Runtime>,
    settings: &Settings,
) -> Result<(lapin::Connection, lapin::Channel), lapin::Error> {
    let connection = lapin::Connection::connect(
        &settings.amqp_uri,
        lapin::ConnectionProperties::default().with_tokio(runtime),
    )
    .await?;
    info!("connected to AMQP server `{}`", settings.amqp_uri);

    let channel = connection.create_channel().await?;
    if !settings.amqp_exchange.is_empty() {
        info!("declaring direct exchange `{}`", settings.amqp_exchange);
        channel
            .exchange_declare(
                &settings.amqp_exchange,
                lapin::ExchangeKind::Direct,
                lapin::options::ExchangeDeclareOptions {
                    passive: false,
                    durable: true,
                    auto_delete: false,
                    internal: false,
                    nowait: false,
                },
                lapin::types::FieldTable::default(),
            )
            .await?;
    }
    Ok((connection, channel))
}

async fn liveness_probe_server(state: Arc<AtomicBool>, settings: &Settings) {
    let mut app = tide::with_state(state);
    let http_probe_port = settings.http_probe_port;
    app.at("/").get(liveness_probe);
    tokio::spawn(async move {
        app.listen(format!("0.0.0.0:{}", http_probe_port))
            .await
            .unwrap_or_else(|err| panic!("unable to perform HTTP liveness probe: `{}`", err))
    });
}

async fn liveness_probe(request: tide::Request<Arc<AtomicBool>>) -> tide::Result {
    let value = request.state().load(Ordering::Relaxed);
    Ok(tide::Response::builder(if value {
        tide::StatusCode::Ok
    } else {
        tide::StatusCode::InternalServerError
    })
    .build())
}

fn setup_logging(debug: bool) -> eyre::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(if debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn main() -> eyre::Result<()> {
    stable_eyre::install()?;
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()?,
    );
    rt.block_on(tokio_main(rt.clone()))?;
    Ok(())
}
