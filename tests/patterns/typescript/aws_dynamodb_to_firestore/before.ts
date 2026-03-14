import {
  DynamoDBClient,
  PutItemCommand,
  GetItemCommand,
  DeleteItemCommand,
  QueryCommand,
} from "@aws-sdk/client-dynamodb";
import { marshall, unmarshall } from "@aws-sdk/util-dynamodb";

const client = new DynamoDBClient({ region: "us-east-1" });
const TABLE_NAME = "users";

interface User {
  userId: string;
  name: string;
  email: string;
}

export async function createUser(user: User): Promise<void> {
  const command = new PutItemCommand({
    TableName: TABLE_NAME,
    Item: marshall(user),
  });
  await client.send(command);
}

export async function getUser(userId: string): Promise<User | null> {
  const command = new GetItemCommand({
    TableName: TABLE_NAME,
    Key: marshall({ userId }),
  });
  const response = await client.send(command);
  if (!response.Item) return null;
  return unmarshall(response.Item) as User;
}

export async function deleteUser(userId: string): Promise<void> {
  const command = new DeleteItemCommand({
    TableName: TABLE_NAME,
    Key: marshall({ userId }),
  });
  await client.send(command);
}

export async function queryUsersByEmail(email: string): Promise<User[]> {
  const command = new QueryCommand({
    TableName: TABLE_NAME,
    IndexName: "email-index",
    KeyConditionExpression: "email = :email",
    ExpressionAttributeValues: marshall({ ":email": email }),
  });
  const response = await client.send(command);
  return (response.Items ?? []).map((item) => unmarshall(item) as User);
}
