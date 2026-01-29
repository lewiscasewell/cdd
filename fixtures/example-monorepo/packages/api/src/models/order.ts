// Non-circular: leaf node
export interface Order {
  id: string;
  userId: string;
  items: string[];
  status: 'pending' | 'completed' | 'cancelled';
}
