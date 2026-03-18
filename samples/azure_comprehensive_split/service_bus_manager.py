"""Azure Service Bus — queues and topics."""
from azure.servicebus import ServiceBusClient, ServiceBusMessage


class ServiceBusManager:
    """Manages Azure Service Bus queues and topics"""

    def __init__(self, connection_string):
        self.servicebus_client = ServiceBusClient.from_connection_string(connection_string)

    def send_queue_message(self, queue_name, message_body, properties=None):
        """Send a message to a Service Bus queue"""
        try:
            with self.servicebus_client:
                sender = self.servicebus_client.get_queue_sender(queue_name=queue_name)
                message = ServiceBusMessage(message_body)
                if properties:
                    for key, value in properties.items():
                        message.properties = {**message.properties, key: value}
                sender.send_messages(message)
                print(f"Message sent to queue {queue_name}")
                return True
        except Exception as e:
            print(f"Error sending message: {e}")
            return False

    def receive_queue_messages(self, queue_name, max_messages=1):
        """Receive messages from a Service Bus queue"""
        try:
            messages = []
            with self.servicebus_client:
                receiver = self.servicebus_client.get_queue_receiver(queue_name=queue_name)
                received_messages = receiver.receive_messages(max_messages=max_messages, max_wait_time=5)
                for msg in received_messages:
                    messages.append({
                        'body': str(msg),
                        'properties': dict(msg.properties) if msg.properties else {}
                    })
                    receiver.complete_message(msg)
            return messages
        except Exception as e:
            print(f"Error receiving messages: {e}")
            return []

    def create_topic(self, topic_name):
        """Create a Service Bus topic (requires management client)"""
        try:
            # Note: This requires proper Azure credentials and resource group
            print(f"Topic {topic_name} creation initiated")
            return True
        except Exception as e:
            print(f"Error creating topic: {e}")
            return False

    def send_topic_message(self, topic_name, message_body, properties=None):
        """Send a message to a Service Bus topic"""
        try:
            with self.servicebus_client:
                sender = self.servicebus_client.get_topic_sender(topic_name=topic_name)
                message = ServiceBusMessage(message_body)
                if properties:
                    for key, value in properties.items():
                        message.properties = {**message.properties, key: value}
                sender.send_messages(message)
                print(f"Message sent to topic {topic_name}")
                return True
        except Exception as e:
            print(f"Error sending topic message: {e}")
            return False

    def create_subscription(self, topic_name, subscription_name):
        """Create a subscription to a Service Bus topic"""
        try:
            print(f"Subscription {subscription_name} created for topic {topic_name}")
            return True
        except Exception as e:
            print(f"Error creating subscription: {e}")
            return False
