import boto3
import uuid
from datetime import datetime

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('Users')

def create_user(email: str, name: str, role: str = 'user') -> dict:
    """Create a new user in DynamoDB."""
    user_id = str(uuid.uuid4())
    item = {
        'user_id': user_id,
        'email': email,
        'name': name,
        'role': role,
        'created_at': datetime.utcnow().isoformat(),
        'active': True
    }
    table.put_item(Item=item)
    return item

def get_user(user_id: str) -> dict:
    """Get a user by ID."""
    response = table.get_item(Key={'user_id': user_id})
    return response.get('Item')

def update_user(user_id: str, updates: dict) -> None:
    """Update user attributes."""
    expression_parts = []
    expression_values = {}
    expression_names = {}
    for key, value in updates.items():
        expression_parts.append(f"#{key} = :{key}")
        expression_values[f":{key}"] = value
        expression_names[f"#{key}"] = key

    table.update_item(
        Key={'user_id': user_id},
        UpdateExpression="SET " + ", ".join(expression_parts),
        ExpressionAttributeValues=expression_values,
        ExpressionAttributeNames=expression_names
    )

def delete_user(user_id: str) -> None:
    """Delete a user."""
    table.delete_item(Key={'user_id': user_id})

def list_active_users() -> list:
    """Scan for all active users."""
    response = table.scan(
        FilterExpression='active = :active',
        ExpressionAttributeValues={':active': True}
    )
    return response.get('Items', [])

def query_users_by_role(role: str) -> list:
    """Query users by role using a GSI."""
    response = table.query(
        IndexName='role-index',
        KeyConditionExpression='role = :role',
        ExpressionAttributeValues={':role': role}
    )
    return response.get('Items', [])
