import { DynamoDBClient, PutItemCommand, GetItemCommand, DeleteItemCommand, QueryCommand } from '@aws-sdk/client-dynamodb';
import { marshall, unmarshall } from '@aws-sdk/util-dynamodb';

const dynamo = new DynamoDBClient({ region: 'us-east-1' });
const TABLE_NAME = 'Orders';

export async function createOrder(order: Record<string, any>): Promise<void> {
  await dynamo.send(new PutItemCommand({
    TableName: TABLE_NAME,
    Item: marshall(order),
  }));
}

export async function getOrder(orderId: string): Promise<Record<string, any> | null> {
  const response = await dynamo.send(new GetItemCommand({
    TableName: TABLE_NAME,
    Key: marshall({ order_id: orderId }),
  }));
  return response.Item ? unmarshall(response.Item) : null;
}

export async function deleteOrder(orderId: string): Promise<void> {
  await dynamo.send(new DeleteItemCommand({
    TableName: TABLE_NAME,
    Key: marshall({ order_id: orderId }),
  }));
}

export async function queryOrdersByCustomer(customerId: string): Promise<Record<string, any>[]> {
  const response = await dynamo.send(new QueryCommand({
    TableName: TABLE_NAME,
    IndexName: 'customer-index',
    KeyConditionExpression: 'customer_id = :cid',
    ExpressionAttributeValues: marshall({ ':cid': customerId }),
  }));
  return (response.Items || []).map(item => unmarshall(item));
}
