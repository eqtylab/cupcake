import type { NextApiRequest, NextApiResponse } from 'next';

interface User {
  id: number;
  name: string;
  email: string;
}

// Mock database - shared with users.ts
// In a real app, this would be imported from a database module
let users: User[] = [
  { id: 1, name: 'Alice Johnson', email: 'alice@example.com' },
  { id: 2, name: 'Bob Smith', email: 'bob@example.com' },
  { id: 3, name: 'Charlie Brown', email: 'charlie@example.com' },
];

export default function handler(
  req: NextApiRequest,
  res: NextApiResponse<User | { error: string }>
) {
  const { id } = req.query;
  const userId = parseInt(id as string, 10);
  
  if (isNaN(userId)) {
    return res.status(400).json({ error: 'Invalid user ID' });
  }

  if (req.method === 'GET') {
    // Get specific user
    const user = users.find(u => u.id === userId);
    if (!user) {
      return res.status(404).json({ error: 'User not found' });
    }
    res.status(200).json(user);
  } else if (req.method === 'DELETE') {
    // Delete user
    const userIndex = users.findIndex(u => u.id === userId);
    if (userIndex === -1) {
      return res.status(404).json({ error: 'User not found' });
    }
    
    const deletedUser = users[userIndex];
    users.splice(userIndex, 1);
    res.status(200).json(deletedUser);
  } else if (req.method === 'PUT') {
    // Update user
    const userIndex = users.findIndex(u => u.id === userId);
    if (userIndex === -1) {
      return res.status(404).json({ error: 'User not found' });
    }
    
    const { name, email } = req.body;
    if (!name || !email) {
      return res.status(400).json({ error: 'Name and email are required' });
    }
    
    users[userIndex] = { ...users[userIndex], name, email };
    res.status(200).json(users[userIndex]);
  } else {
    res.setHeader('Allow', ['GET', 'DELETE', 'PUT']);
    res.status(405).json({ error: `Method ${req.method} not allowed` });
  }
}