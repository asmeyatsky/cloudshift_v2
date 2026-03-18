"""Azure Storage Queue."""


class QueueStorageManager:
    """Manages Azure Storage Queue operations"""

    def __init__(self, connection_string):
        from azure.storage.queue import QueueServiceClient
        self.queue_service_client = QueueServiceClient.from_connection_string(connection_string)

    def create_queue(self, queue_name):
        """Create a storage queue"""
        try:
            queue_client = self.queue_service_client.create_queue(queue_name)
            print(f"Queue {queue_name} created successfully")
            return queue_client
        except Exception as e:
            print(f"Error creating queue: {e}")
            return None

    def send_message(self, queue_name, message_text):
        """Send a message to the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            queue_client.send_message(message_text)
            print(f"Message sent to queue {queue_name}")
            return True
        except Exception as e:
            print(f"Error sending message: {e}")
            return False

    def receive_messages(self, queue_name, max_messages=1):
        """Receive messages from the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            messages = queue_client.receive_messages(max_messages=max_messages)
            return list(messages)
        except Exception as e:
            print(f"Error receiving messages: {e}")
            return []

    def delete_message(self, queue_name, message_id, pop_receipt):
        """Delete a message from the queue"""
        try:
            queue_client = self.queue_service_client.get_queue_client(queue_name)
            queue_client.delete_message(message_id, pop_receipt)
            print("Message deleted successfully")
            return True
        except Exception as e:
            print(f"Error deleting message: {e}")
            return False
