"""DynamoDB — split from aws_comprehensive_example.py."""
import boto3
import os
from botocore.exceptions import ClientError

class DynamoDBManager:
    """Manages DynamoDB table operations"""
    
    def __init__(self, region_name='us-east-1'):
        self.dynamodb = boto3.resource('dynamodb', region_name=region_name)
        self.dynamodb_client = boto3.client('dynamodb', region_name=region_name)
        self.table_name = os.environ.get('DYNAMODB_TABLE', 'UserData')
    
    def create_table(self, table_name, partition_key, sort_key=None):
        """Create a DynamoDB table"""
        try:
            key_schema = [
                {'AttributeName': partition_key, 'KeyType': 'HASH'}
            ]
            attribute_definitions = [
                {'AttributeName': partition_key, 'AttributeType': 'S'}
            ]
            
            if sort_key:
                key_schema.append({'AttributeName': sort_key, 'KeyType': 'RANGE'})
                attribute_definitions.append({'AttributeName': sort_key, 'AttributeType': 'S'})
            
            table = self.dynamodb.create_table(
                TableName=table_name,
                KeySchema=key_schema,
                AttributeDefinitions=attribute_definitions,
                BillingMode='PAY_PER_REQUEST'
            )
            table.wait_until_exists()
            print(f"Table {table_name} created successfully")
            return table
        except ClientError as e:
            print(f"Error creating table: {e}")
            return None
    
    def put_item(self, table_name, item):
        """Put an item into DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.put_item(Item=item)
            print(f"Item inserted into {table_name}")
            return response
        except ClientError as e:
            print(f"Error putting item: {e}")
            return None
    
    def get_item(self, table_name, key):
        """Get an item from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.get_item(Key=key)
            if 'Item' in response:
                return response['Item']
            return None
        except ClientError as e:
            print(f"Error getting item: {e}")
            return None
    
    def query_items(self, table_name, partition_key_value, index_name=None):
        """Query items from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            if index_name:
                response = table.query(
                    IndexName=index_name,
                    KeyConditionExpression='partition_key = :pk',
                    ExpressionAttributeValues={':pk': partition_key_value}
                )
            else:
                response = table.query(
                    KeyConditionExpression='id = :pk',
                    ExpressionAttributeValues={':pk': partition_key_value}
                )
            return response.get('Items', [])
        except ClientError as e:
            print(f"Error querying items: {e}")
            return []
    
    def scan_table(self, table_name, filter_expression=None):
        """Scan all items in DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            if filter_expression:
                response = table.scan(FilterExpression=filter_expression)
            else:
                response = table.scan()
            return response.get('Items', [])
        except ClientError as e:
            print(f"Error scanning table: {e}")
            return []
    
    def update_item(self, table_name, key, update_expression, expression_attribute_values):
        """Update an item in DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.update_item(
                Key=key,
                UpdateExpression=update_expression,
                ExpressionAttributeValues=expression_attribute_values,
                ReturnValues='UPDATED_NEW'
            )
            return response
        except ClientError as e:
            print(f"Error updating item: {e}")
            return None
    
    def delete_item(self, table_name, key):
        """Delete an item from DynamoDB table"""
        try:
            table = self.dynamodb.Table(table_name)
            response = table.delete_item(Key=key)
            print(f"Item deleted from {table_name}")
            return response
        except ClientError as e:
            print(f"Error deleting item: {e}")
            return None
