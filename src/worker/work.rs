//! Worker working.

use tokio::{
    task,
    time::{self, Duration},
};

use crate::digitalocean::models::app::App;
use crate::worker::Worker;

impl Worker {
    /// Gets to work!
    pub async fn work(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.interval));

        loop {
            interval.tick().await;

            log::debug!("checking for new deployments...");

            if let Err(e) = work(self).await {
                log::warn!("{}", e);
            }            
        }
    }
}

async fn work(worker: &Worker) -> anyhow::Result<()> {
    let apps = worker.digitalocean.apps().get().await?;

    let mut handles = Vec::new();

    for app in apps {
        let w = worker.clone();

        let handle = task::spawn(async move {
            if let Err(e) = task(w, app).await {
                log::warn!("{}", e);
            }
        });

        handles.push(handle);
    }

    // Await for all tasks
    for handle in handles {
        handle.await?;
    }

    Ok(())
}

async fn task(worker: Worker, app: App) -> anyhow::Result<()> {
    // Check if the table exists
    if !worker.database.table(&app.id).exists() {
        log::info!("a new App ({}) has been detected", &app.id);

        // Create a table
        worker.database.table(&app.id).create().await?;

        // Get deployments
        let deployments = worker.digitalocean.deployments().get(&app).await?;

        // Write data to the table
        let data: Vec<&str> = deployments.iter().map(|d| d.id.as_str()).collect();
        worker.database.table(&app.id).write(data).await?;

        // Telegram!!!!!

        return Ok(());
    }

    // Get deployments
    let deployments = worker.digitalocean.deployments().get(&app).await?;

    // Get deployments from table
    let deployments_current = worker.database.table(&app.id).read().await?;

    // Search for new deployments
    for deployment in deployments.iter() {
        if !deployments_current.contains(&deployment.id) {
            log::info!("A new deployment ({}) has been detected", &deployment.id);

            // Telegram!!!!!
        }
    }

    // Write data to the table
    let data: Vec<&str> = deployments.iter().map(|d| d.id.as_str()).collect();
    worker.database.table(&app.id).write(data).await?;

    Ok(())
}
