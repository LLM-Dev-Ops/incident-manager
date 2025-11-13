#!/usr/bin/env python3
"""
Python WebSocket Client Example

This example demonstrates how to connect to the LLM Incident Manager
WebSocket API and subscribe to real-time incident updates using Python.

Requirements:
    pip install websockets gql[websockets]
"""

import asyncio
import logging
import os
import signal
from typing import Callable, Dict, Any
from gql import Client, gql
from gql.transport.websockets import WebsocketsTransport

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class IncidentMonitor:
    """WebSocket client for monitoring incidents in real-time"""

    def __init__(self, ws_url: str, auth_token: str):
        """
        Initialize the incident monitor

        Args:
            ws_url: WebSocket URL (e.g., 'ws://localhost:8080/graphql/ws')
            auth_token: JWT authentication token
        """
        self.transport = WebsocketsTransport(
            url=ws_url,
            init_payload={'Authorization': f'Bearer {auth_token}'},
            subprotocols=[WebsocketsTransport.GRAPHQLWS_SUBPROTOCOL]
        )
        self.client = Client(
            transport=self.transport,
            fetch_schema_from_transport=False
        )
        self.running = False

    async def subscribe_to_critical_incidents(
        self,
        handler: Callable[[Dict[str, Any]], None]
    ):
        """
        Subscribe to critical incidents (P0 and P1)

        Args:
            handler: Callback function to handle incidents
        """
        subscription = gql('''
            subscription {
              criticalIncidents {
                id
                title
                description
                severity
                state
                createdAt
                affectedResources
              }
            }
        ''')

        logger.info("Subscribing to critical incidents...")

        try:
            async with self.client as session:
                async for result in session.subscribe(subscription):
                    incident = result['criticalIncidents']
                    logger.info(f"🚨 Critical incident: {incident['id']}")
                    handler(incident)

                    if not self.running:
                        break
        except Exception as e:
            logger.error(f"Critical incidents subscription error: {e}")

    async def subscribe_to_incident_updates(
        self,
        severities: list[str],
        handler: Callable[[Dict[str, Any]], None]
    ):
        """
        Subscribe to incident updates with severity filter

        Args:
            severities: List of severity levels (e.g., ['P0', 'P1'])
            handler: Callback function to handle updates
        """
        subscription = gql('''
            subscription IncidentUpdates($severities: [Severity!]) {
              incidentUpdates(severities: $severities, activeOnly: true) {
                updateType
                incidentId
                timestamp
              }
            }
        ''')

        logger.info(f"Subscribing to incident updates (severities: {severities})...")

        try:
            async with self.client as session:
                async for result in session.subscribe(
                    subscription,
                    variable_values={'severities': severities}
                ):
                    update = result['incidentUpdates']
                    logger.info(f"📢 Update: {update['updateType']} - {update['incidentId']}")
                    handler(update)

                    if not self.running:
                        break
        except Exception as e:
            logger.error(f"Incident updates subscription error: {e}")

    async def subscribe_to_new_incidents(
        self,
        severities: list[str],
        handler: Callable[[Dict[str, Any]], None]
    ):
        """
        Subscribe to new incidents

        Args:
            severities: List of severity levels to filter by
            handler: Callback function to handle new incidents
        """
        subscription = gql('''
            subscription NewIncidents($severities: [Severity!]) {
              newIncidents(severities: $severities) {
                id
                title
                description
                severity
                state
                incidentType
                source
                createdAt
                affectedResources
              }
            }
        ''')

        logger.info(f"Subscribing to new incidents (severities: {severities})...")

        try:
            async with self.client as session:
                async for result in session.subscribe(
                    subscription,
                    variable_values={'severities': severities}
                ):
                    incident = result['newIncidents']
                    logger.info(f"🆕 New incident: {incident['id']} - {incident['title']}")
                    handler(incident)

                    if not self.running:
                        break
        except Exception as e:
            logger.error(f"New incidents subscription error: {e}")

    def stop(self):
        """Stop all subscriptions"""
        logger.info("Stopping incident monitor...")
        self.running = False


# Notification handlers (implement these based on your needs)
def handle_critical_incident(incident: Dict[str, Any]):
    """Handle critical incident notification"""
    print(f"\n{'='*60}")
    print(f"🚨 CRITICAL INCIDENT ALERT")
    print(f"{'='*60}")
    print(f"ID:          {incident['id']}")
    print(f"Title:       {incident['title']}")
    print(f"Severity:    {incident['severity']}")
    print(f"State:       {incident['state']}")
    print(f"Created:     {incident['createdAt']}")
    print(f"Affected:    {', '.join(incident['affectedResources'])}")
    print(f"{'='*60}\n")

    # Send to PagerDuty
    # send_pagerduty_alert(incident)

    # Send to Slack
    # send_slack_alert(incident)


def handle_incident_update(update: Dict[str, Any]):
    """Handle incident update notification"""
    print(f"📢 {update['updateType']}: {update['incidentId']} at {update['timestamp']}")

    # Update dashboard
    # update_dashboard(update)


def handle_new_incident(incident: Dict[str, Any]):
    """Handle new incident notification"""
    print(f"🆕 New {incident['severity']} incident: {incident['title']}")

    # Create ticket
    # create_jira_ticket(incident)


async def main():
    """Main execution function"""
    ws_url = os.getenv('WS_URL', 'ws://localhost:8080/graphql/ws')
    auth_token = os.getenv('AUTH_TOKEN', 'YOUR_JWT_TOKEN')

    monitor = IncidentMonitor(ws_url, auth_token)
    monitor.running = True

    # Set up graceful shutdown
    def signal_handler(sig, frame):
        logger.info("Shutdown signal received")
        monitor.stop()

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    logger.info("Starting incident monitor...")
    logger.info("Press Ctrl+C to stop")

    # Run all subscriptions concurrently
    try:
        await asyncio.gather(
            monitor.subscribe_to_critical_incidents(handle_critical_incident),
            monitor.subscribe_to_incident_updates(['P0', 'P1'], handle_incident_update),
            monitor.subscribe_to_new_incidents(['P0', 'P1', 'P2'], handle_new_incident),
            return_exceptions=True
        )
    except Exception as e:
        logger.error(f"Error in main loop: {e}")
    finally:
        logger.info("Incident monitor stopped")


if __name__ == '__main__':
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        logger.info("Interrupted by user")
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        raise
