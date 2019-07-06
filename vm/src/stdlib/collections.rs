use crate::function::OptionalArg;
use crate::obj::{objbool, objsequence, objtype::PyClassRef};
use crate::pyobject::{IdProtocol, PyClassImpl, PyIterable, PyObjectRef, PyRef, PyResult, PyValue};
use crate::vm::ReprGuard;
use crate::VirtualMachine;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::VecDeque;

#[pyclass(name = "deque")]
#[derive(Debug, Clone)]
struct PyDeque {
    deque: RefCell<VecDeque<PyObjectRef>>,
    maxlen: Option<usize>,
}

impl PyValue for PyDeque {
    fn class(vm: &VirtualMachine) -> PyClassRef {
        vm.class("_collections", "deque")
    }
}


#[pyimpl]
impl PyDeque {
    #[pymethod(name = "__new__")]
    fn new(
        cls: PyClassRef,
        iter: OptionalArg<PyIterable>,
        maxlen: OptionalArg<Option<usize>>,
        vm: &VirtualMachine,
    ) -> PyResult<PyRef<Self>> {
        let deque = if let OptionalArg::Present(iter) = iter {
            iter.iter(vm)?.collect::<Result<_, _>>()?
        } else {
            VecDeque::new()
        };
        PyDeque {
            deque: RefCell::new(deque),
            maxlen: maxlen.into_option().and_then(|x| x),
        }
        .into_ref_with_type(vm, cls)
    }

    #[pymethod]
    fn append(&self, obj: PyObjectRef, _vm: &VirtualMachine) {
        let mut deque = self.deque.borrow_mut();
        if let Some(maxlen) = self.maxlen {
            if deque.len() == maxlen {
                deque.pop_front();
            }
        }
        deque.push_back(obj);
    }

    #[pymethod]
    fn appendleft(&self, obj: PyObjectRef, _vm: &VirtualMachine) {
        let mut deque = self.deque.borrow_mut();
        if let Some(maxlen) = self.maxlen {
            if deque.len() == maxlen {
                deque.pop_back();
            }
        }
        deque.push_front(obj);
    }

    #[pymethod]
    fn clear(&self, _vm: &VirtualMachine) {
        self.deque.borrow_mut().clear()
    }

    #[pymethod]
    fn copy(&self, _vm: &VirtualMachine) -> Self {
        self.clone()
    }

    #[pymethod]
    fn count(&self, obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<usize> {
        let mut count = 0;
        for elem in self.deque.borrow().iter() {
            if objbool::boolval(vm, vm._eq(elem.clone(), obj.clone())?)? {
                count += 1;
            }
        }
        Ok(count)
    }

    #[pymethod]
    fn extend(&self, iter: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        // TODO: use length_hint here and for extendleft
        for elem in iter.iter(vm)? {
            self.append(elem?, vm);
        }
        Ok(())
    }

    #[pymethod]
    fn extendleft(&self, iter: PyIterable, vm: &VirtualMachine) -> PyResult<()> {
        for elem in iter.iter(vm)? {
            self.appendleft(elem?, vm);
        }
        Ok(())
    }

    #[pymethod]
    fn index(
        &self,
        obj: PyObjectRef,
        start: OptionalArg<usize>,
        stop: OptionalArg<usize>,
        vm: &VirtualMachine,
    ) -> PyResult<usize> {
        let deque = self.deque.borrow();
        let start = start.unwrap_or(0);
        let stop = stop.unwrap_or_else(|| deque.len());
        for (i, elem) in deque.iter().skip(start).take(stop - start).enumerate() {
            if objbool::boolval(vm, vm._eq(elem.clone(), obj.clone())?)? {
                return Ok(i);
            }
        }
        Err(vm.new_value_error(
            vm.to_repr(&obj)
                .map(|repr| format!("{} is not in deque", repr))
                .unwrap_or_else(|_| String::new()),
        ))
    }

    #[pymethod]
    fn insert(&self, idx: i32, obj: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
        let mut deque = self.deque.borrow_mut();

        if let Some(maxlen) = self.maxlen {
            if deque.len() == maxlen {
                return Err(vm.new_index_error("deque already at its maximum size".to_string()));
            }
        }

        let idx = if idx < 0 {
            if -idx as usize > deque.len() {
                0
            } else {
                deque.len() - ((-idx) as usize)
            }
        } else if idx as usize >= deque.len() {
            deque.len() - 1
        } else {
            idx as usize
        };

        deque.insert(idx, obj);

        Ok(())
    }

    #[pymethod]
    fn pop(&self, vm: &VirtualMachine) -> PyResult {
        self.deque
            .borrow_mut()
            .pop_back()
            .ok_or_else(|| vm.new_index_error("pop from an empty deque".to_string()))
    }

    #[pymethod]
    fn popleft(&self, vm: &VirtualMachine) -> PyResult {
        self.deque
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| vm.new_index_error("pop from an empty deque".to_string()))
    }

    #[pymethod]
    fn remove(&self, obj: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        let mut deque = self.deque.borrow_mut();
        let mut idx = None;
        for (i, elem) in deque.iter().enumerate() {
            if objbool::boolval(vm, vm._eq(elem.clone(), obj.clone())?)? {
                idx = Some(i);
                break;
            }
        }
        idx.map(|idx| deque.remove(idx).unwrap())
            .ok_or_else(|| vm.new_value_error("deque.remove(x): x not in deque".to_string()))
    }

    #[pymethod]
    fn reverse(&self, _vm: &VirtualMachine) {
        self.deque
            .replace_with(|deque| deque.iter().cloned().rev().collect());
    }

    #[pymethod]
    fn rotate(&self, mid: OptionalArg<isize>, _vm: &VirtualMachine) {
        let mut deque = self.deque.borrow_mut();
        let mid = mid.unwrap_or(1);
        // TODO: once `vecdeque_rotate` lands, use that instead
        if mid < 0 {
            for _ in 0..-mid {
                if let Some(popped_front) = deque.pop_front() {
                    deque.push_back(popped_front);
                }
            }
        } else {
            for _ in 0..mid {
                if let Some(popped_back) = deque.pop_back() {
                    deque.push_front(popped_back);
                }
            }
        }
    }

    #[pyproperty]
    fn maxlen(&self, _vm: &VirtualMachine) -> Option<usize> {
        self.maxlen
    }

    #[pymethod(name = "__repr__")]
    fn repr(zelf: PyRef<Self>, vm: &VirtualMachine) -> PyResult<String> {
        let repr = if let Some(_guard) = ReprGuard::enter(zelf.as_object()) {
            let elements = zelf
                .deque
                .borrow()
                .iter()
                .map(|obj| vm.to_repr(obj))
                .collect::<Result<Vec<_>, _>>()?;
            let maxlen = zelf
                .maxlen
                .map(|maxlen| format!(", maxlen={}", maxlen))
                .unwrap_or_default();
            format!("deque([{}]{})", elements.into_iter().format(", "), maxlen)
        } else {
            "[...]".to_string()
        };
        Ok(repr)
    }

    #[pymethod(name = "__eq__")]
    fn eq(zelf: PyRef<Self>, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if zelf.as_object().is(&other) {
            return Ok(vm.new_bool(true));
        }

        let other = match_class!(other,
            other @ Self => other,
            _ => return Ok(vm.ctx.not_implemented()),
        );

        let lhs: &VecDeque<_> = &zelf.deque.borrow();
        let rhs: &VecDeque<_> = &other.deque.borrow();

        let eq = objsequence::seq_equal(vm, lhs, rhs)?;
        Ok(vm.new_bool(eq))
    }

    #[pymethod(name = "__lt__")]
    fn lt(zelf: PyRef<Self>, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if zelf.as_object().is(&other) {
            return Ok(vm.new_bool(true));
        }

        let other = match_class!(other,
            other @ Self => other,
            _ => return Ok(vm.ctx.not_implemented()),
        );

        let lhs: &VecDeque<_> = &zelf.deque.borrow();
        let rhs: &VecDeque<_> = &other.deque.borrow();

        let eq = objsequence::seq_lt(vm, lhs, rhs)?;
        Ok(vm.new_bool(eq))
    }

    #[pymethod(name = "__gt__")]
    fn gt(zelf: PyRef<Self>, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if zelf.as_object().is(&other) {
            return Ok(vm.new_bool(true));
        }

        let other = match_class!(other,
            other @ Self => other,
            _ => return Ok(vm.ctx.not_implemented()),
        );

        let lhs: &VecDeque<_> = &zelf.deque.borrow();
        let rhs: &VecDeque<_> = &other.deque.borrow();

        let eq = objsequence::seq_gt(vm, lhs, rhs)?;
        Ok(vm.new_bool(eq))
    }

    #[pymethod(name = "__le__")]
    fn le(zelf: PyRef<Self>, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if zelf.as_object().is(&other) {
            return Ok(vm.new_bool(true));
        }

        let other = match_class!(other,
            other @ Self => other,
            _ => return Ok(vm.ctx.not_implemented()),
        );

        let lhs: &VecDeque<_> = &zelf.deque.borrow();
        let rhs: &VecDeque<_> = &other.deque.borrow();

        let eq = objsequence::seq_le(vm, lhs, rhs)?;
        Ok(vm.new_bool(eq))
    }

    #[pymethod(name = "__ge__")]
    fn ge(zelf: PyRef<Self>, other: PyObjectRef, vm: &VirtualMachine) -> PyResult {
        if zelf.as_object().is(&other) {
            return Ok(vm.new_bool(true));
        }

        let other = match_class!(other,
            other @ Self => other,
            _ => return Ok(vm.ctx.not_implemented()),
        );

        let lhs: &VecDeque<_> = &zelf.deque.borrow();
        let rhs: &VecDeque<_> = &other.deque.borrow();

        let eq = objsequence::seq_ge(vm, lhs, rhs)?;
        Ok(vm.new_bool(eq))
    }
}

pub fn make_module(vm: &VirtualMachine) -> PyObjectRef {
    py_module!(vm, "_collections", {
        "deque" => PyDeque::make_class(&vm.ctx),
    })
}
