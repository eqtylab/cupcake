import { useState, useEffect } from 'react';
import Head from 'next/head';

interface User {
  id: number;
  name: string;
  email: string;
}

export default function Home() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchUsers();
  }, []);

  const fetchUsers = async () => {
    try {
      const response = await fetch('/api/users');
      if (!response.ok) {
        throw new Error('Failed to fetch users');
      }
      const data = await response.json();
      setUsers(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const deleteUser = async (id: number) => {
    try {
      const response = await fetch(`/api/users/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok) {
        throw new Error('Failed to delete user');
      }
      setUsers(users.filter(user => user.id !== id));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete user');
    }
  };

  return (
    <>
      <Head>
        <title>Cupcake Test App</title>
        <meta name="description" content="Test application for Cupcake policy testing" />
      </Head>

      <main className="container">
        <h1>Cupcake Policy Test Application</h1>
        
        <p>This is a sample Next.js application for testing Cupcake policies with frontend code.</p>

        <section>
          <h2>User Management</h2>
          
          {loading && <p>Loading users...</p>}
          {error && <p className="error">Error: {error}</p>}
          
          {!loading && !error && (
            <div>
              <h3>Users ({users.length})</h3>
              <ul>
                {users.map(user => (
                  <li key={user.id}>
                    {user.name} ({user.email})
                    <button 
                      onClick={() => deleteUser(user.id)}
                      className="delete-btn"
                    >
                      Delete
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </section>
      </main>

      <style jsx>{`
        .container {
          max-width: 800px;
          margin: 0 auto;
          padding: 2rem;
          font-family: -apple-system, BlinkMacSystemFont, sans-serif;
        }
        
        .error {
          color: red;
          font-weight: bold;
        }
        
        .delete-btn {
          margin-left: 1rem;
          background: red;
          color: white;
          border: none;
          padding: 0.25rem 0.5rem;
          cursor: pointer;
          border-radius: 3px;
        }
        
        .delete-btn:hover {
          background: darkred;
        }
        
        ul {
          list-style: none;
          padding: 0;
        }
        
        li {
          padding: 0.5rem;
          border-bottom: 1px solid #eee;
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
      `}</style>
    </>
  );
}