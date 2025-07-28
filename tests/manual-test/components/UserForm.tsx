import { useState } from 'react';

interface UserFormProps {
  onSubmit: (user: { name: string; email: string }) => void;
  onCancel?: () => void;
  initialValues?: { name: string; email: string };
  submitLabel?: string;
}

export default function UserForm({ 
  onSubmit, 
  onCancel, 
  initialValues = { name: '', email: '' },
  submitLabel = 'Create User' 
}: UserFormProps) {
  const [name, setName] = useState(initialValues.name);
  const [email, setEmail] = useState(initialValues.email);
  const [errors, setErrors] = useState<{ name?: string; email?: string }>({});

  const validateForm = () => {
    const newErrors: { name?: string; email?: string } = {};
    
    if (!name.trim()) {
      newErrors.name = 'Name is required';
    }
    
    if (!email.trim()) {
      newErrors.email = 'Email is required';
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      newErrors.email = 'Please enter a valid email address';
    }
    
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    if (validateForm()) {
      onSubmit({ name: name.trim(), email: email.trim() });
      // Reset form after successful submission
      setName('');
      setEmail('');
      setErrors({});
    }
  };

  return (
    <form onSubmit={handleSubmit} className="user-form">
      <div className="form-group">
        <label htmlFor="name">Name:</label>
        <input
          type="text"
          id="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className={errors.name ? 'error' : ''}
          placeholder="Enter user name"
        />
        {errors.name && <span className="error-message">{errors.name}</span>}
      </div>

      <div className="form-group">
        <label htmlFor="email">Email:</label>
        <input
          type="email"
          id="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          className={errors.email ? 'error' : ''}
          placeholder="Enter email address"
        />
        {errors.email && <span className="error-message">{errors.email}</span>}
      </div>

      <div className="form-actions">
        <button type="submit" className="submit-btn">
          {submitLabel}
        </button>
        {onCancel && (
          <button type="button" onClick={onCancel} className="cancel-btn">
            Cancel
          </button>
        )}
      </div>

      <style jsx>{`
        .user-form {
          background: #f9f9f9;
          padding: 1.5rem;
          border-radius: 8px;
          margin: 1rem 0;
        }
        
        .form-group {
          margin-bottom: 1rem;
        }
        
        label {
          display: block;
          margin-bottom: 0.5rem;
          font-weight: bold;
        }
        
        input {
          width: 100%;
          padding: 0.5rem;
          border: 1px solid #ddd;
          border-radius: 4px;
          font-size: 1rem;
        }
        
        input.error {
          border-color: red;
        }
        
        .error-message {
          color: red;
          font-size: 0.875rem;
          margin-top: 0.25rem;
          display: block;
        }
        
        .form-actions {
          display: flex;
          gap: 1rem;
        }
        
        .submit-btn {
          background: #007bff;
          color: white;
          border: none;
          padding: 0.75rem 1.5rem;
          border-radius: 4px;
          cursor: pointer;
          font-size: 1rem;
        }
        
        .submit-btn:hover {
          background: #0056b3;
        }
        
        .cancel-btn {
          background: #6c757d;
          color: white;
          border: none;
          padding: 0.75rem 1.5rem;
          border-radius: 4px;
          cursor: pointer;
          font-size: 1rem;
        }
        
        .cancel-btn:hover {
          background: #545b62;
        }
      `}</style>
    </form>
  );
}