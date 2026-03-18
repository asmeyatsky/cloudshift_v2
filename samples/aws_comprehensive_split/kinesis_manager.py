"""Split from aws_comprehensive_example.py."""
import boto3
from botocore.exceptions import ClientError

class KinesisManager:
    """Manages AWS Kinesis streams"""
    
    def __init__(self, region_name='us-east-1'):
        self.kinesis_client = boto3.client('kinesis', region_name=region_name)
    
    def create_stream(self, stream_name, shard_count=1):
        """Create a Kinesis stream"""
        try:
            self.kinesis_client.create_stream(
                StreamName=stream_name,
                ShardCount=shard_count
            )
            print(f"Kinesis stream {stream_name} created")
            return True
        except ClientError as e:
            print(f"Error creating stream: {e}")
            return False
    
    def put_record(self, stream_name, data, partition_key):
        """Put a record into a Kinesis stream"""
        try:
            response = self.kinesis_client.put_record(
                StreamName=stream_name,
                Data=data.encode('utf-8'),
                PartitionKey=partition_key
            )
            print(f"Record put into stream {stream_name}")
            return response
        except ClientError as e:
            print(f"Error putting record: {e}")
            return None
    
    def get_records(self, shard_iterator, limit=10):
        """Get records from a Kinesis stream"""
        try:
            response = self.kinesis_client.get_records(
                ShardIterator=shard_iterator,
                Limit=limit
            )
            return response.get('Records', [])
        except ClientError as e:
            print(f"Error getting records: {e}")
            return []
    
    def get_shard_iterator(self, stream_name, shard_id, shard_iterator_type='TRIM_HORIZON'):
        """Get a shard iterator for reading records"""
        try:
            response = self.kinesis_client.get_shard_iterator(
                StreamName=stream_name,
                ShardId=shard_id,
                ShardIteratorType=shard_iterator_type
            )
            return response['ShardIterator']
        except ClientError as e:
            print(f"Error getting shard iterator: {e}")
            return None
