use std::collections::HashSet;
use std::sync::{Condvar, Mutex};

/// Data structure used to allocate a port on demand in a multithreaded context.
pub struct PortAllocator{
    pool: (Mutex<Vec<u16>>,Condvar),
    first_port: u16,
    last_port: u16,
}

impl PortAllocator {
    /// Creates a new port allocator that will use a certain range of ports
    pub fn new(first_port: u16, last_port: u16) -> Self{
        assert!(first_port <= last_port);

        let pool = (first_port..=last_port).collect::<Vec<u16>>();

        Self{
            pool: (Mutex::new(pool),Condvar::new()),
            first_port,
            last_port,
        }

    }

    /// Tries to allocate a new port and blocks the current thread if there is no free port.
    pub fn alloc(&self) -> u16{

        let (mutex,condvar) = &self.pool;
        let mut vec_guard = mutex.lock().unwrap();

        if vec_guard.is_empty(){
            vec_guard = condvar.wait(vec_guard).unwrap();
        }

        vec_guard.pop().unwrap()

    }

    /// Will deallocate the given port by pushing it back in the vector. Will notify one thread that there is a new free port.
    /// Will panic if the given port is not in the given range.
    /// Will not panic if the deallocated port was already free.
    pub fn dealloc(&self, port: u16){
        assert!(port >= self.first_port && self.last_port >= port);

        let (mutex,condvar) = &self.pool;
        let mut vec_guard = mutex.lock().unwrap();

        vec_guard.push(port);
        condvar.notify_one();

    }

    /// Returns the current port pool size.
    pub fn pool_size(&self) -> usize{
        let (mutex,condvar) = &self.pool;
        mutex.lock().unwrap().len()
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;

    #[test]
    fn test_port_allocator_1(){
        let allocator = PortAllocator::new(1,2);
        let port_1 = allocator.alloc();
        let port_2 = allocator.alloc();

        assert_ne!(port_1, port_2);
        assert_eq!(allocator.pool_size(),0);

        allocator.dealloc(port_1);
        assert_eq!(allocator.pool_size(),1);

        allocator.dealloc(port_2);
        assert_eq!(allocator.pool_size(),2);
    }

    #[test]
    #[should_panic]
    fn test_port_allocator_2(){
        PortAllocator::new(100,20);
    }

    #[test]
    fn test_port_allocator_3(){
        let allocator = Arc::new(PortAllocator::new(1,5));

        let mut ports = (1..=5).collect::<Vec<u16>>();
        ports.sort();



        let mut handles = Vec::with_capacity(5);

        for i in 1..=5{

            let allocator_clone = allocator.clone();

            let handle = thread::spawn(move || {
                return allocator_clone.alloc();
            });

            handles.push(handle);

        }

        let mut ports_after = handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<u16>>();
        ports_after.sort();

        assert_eq!(ports, ports_after);
    }
}
