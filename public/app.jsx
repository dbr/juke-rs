class ErrorBoundary extends React.Component {
    constructor(props) {
      super(props);
      this.state = { error: null, errorInfo: null };
    }
  
    componentDidCatch(error, info) {
        this.setState({
            error: error,
            errorInfo: errorInfo
        });
    }

    clear(event) {
        event.preventDefault();
        this.setState({error: null, errorInfo: null});
    }
  
    render() {
        if (this.state.errorInfo) {
            // Error path
            return (
              <div>
                <h2>Something went wrong.</h2>
                <details style={{ whiteSpace: 'pre-wrap' }}>
                  {this.state.error && this.state.error.toString()}
                  <br />
                  {this.state.errorInfo.componentStack}
                </details>
                <a href="#" onClick={this.clear}>Cool</a>
              </div>
            );
          }
        return this.props.children;
    }
  }

class PlaybackStatus extends React.Component {
    constructor(props) {
        super(props);
        this.state = {info: undefined};
    }
    componentDidMount() {
        const sock = new WebSocket('ws://localhost:8081/ws', 'juke');
        sock.addEventListener('open', function (event) {
            sock.send('status');
        });
        sock.addEventListener('message', function (event) {
            let d = event.data;
            this.update(JSON.parse(d));
        }.bind(this));

        // Trigger routine update
        this.timer = setInterval(function(){sock.send('status')}.bind(this), 1000);
    }
    componentWillUnmount() {
        clearInterval(this.timer);
    }
    update(data) {
        this.setState({"info": data});
    }
    render() {
        if (this.state.info === undefined) {
            return <div>[loading...!]</div>;
        }
        return (
            <span>
                <p>
                    Playing: <b>{this.state.info.Status.song.title}</b> by <b>{this.state.info.Status.song.artist}</b>
                    <i> ({this.state.info.Status.state})</i>
                </p>
            </span>
        )
    }
}

class SearchWidget extends React.Component {
    constructor(props) {
        super(props);
        this.state = { value: '', data: [] };

        this.handleChange = this.handleChange.bind(this);
        this.handleSubmit = this.handleSubmit.bind(this);
    }
    clearResults(){
        this.setState({data: []});
    }
    handleChange(event) {
        this.clearResults();
        this.setState({ value: event.target.value });
    }
    handleSubmit(event) {
        self = this;
        event.preventDefault();
        this.clearResults();

        var u = new URL("http://localhost:8081/search/track/" + encodeURIComponent(this.state.value));
        fetch(u).then(function(resp){
            return resp.json(); // FIXME: Handle error
        }).then(function(d){
            self.setState({data: d});
        })
    }

    play(event) {
        event.preventDefault();
        let spotify_uri = event.target.value;
        console.log('k', spotify_uri)
        var u = new URL("http://localhost:8081/api/request/" + encodeURIComponent(spotify_uri));
        fetch(u).then(function(resp){
            return resp.json(); // FIXME: Handle error
        }).then(function(d){
            console.log(d);
        })
    }

    render() {
        // Format search results if any
        if (this.state.data.Search && this.state.data.Search.items.length > 0){
            var sr = <ul>
                {this.state.data.Search.items.map(
                    (x) => <li key={x.spotify_uri}>
                        <a href="#" onClick={this.play} value={x.spotify_uri}>
                            <b>{x.name}</b> by <b>{x.artists}</b> ({x.spotify_uri}
                        </a>
                    </li>)}
            </ul>
        } else {
            var sr = <div></div>;
        }

        // Main form + results
        return (
            <div>
            <form onSubmit={this.handleSubmit}>
                <label>
                    Name:
            <input type="text" value={this.state.value} onChange={this.handleChange} />
                </label>
                <input type="submit" value="Submit" />
            </form>
            {sr}
            </div>
        );
    }
}

ReactDOM.render(
    <ErrorBoundary>
        <PlaybackStatus />
        <SearchWidget />
    </ErrorBoundary>,
    document.getElementById("app")
);